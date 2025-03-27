use anyhow::{anyhow, Context};
use async_lsp::{
    client_monitor::ClientProcessMonitorLayer,
    concurrency::ConcurrencyLayer,
    lsp_types::{
        DidChangeConfigurationParams, DidChangeTextDocumentParams, DidOpenTextDocumentParams,
        InitializeParams, InitializeResult, ServerCapabilities, TextDocumentContentChangeEvent,
        TextDocumentSyncCapability::Kind, TextDocumentSyncKind,
    },
    panic::CatchUnwindLayer,
    router::Router,
    server::LifecycleLayer,
    tracing::TracingLayer,
    ClientSocket, LanguageClient as _, LanguageServer, ResponseError,
};
use codlab::{
    change_event_to_workspace_edit, common::init_logger, messages::Message,
    peekable_channel::PeekableReceiver,
};
use futures::{future::BoxFuture, stream::SplitSink, SinkExt, StreamExt as _, TryStreamExt};
use std::{
    ops::ControlFlow,
    sync::{
        mpsc::{self, Sender},
        Arc,
    },
};
use tokio::sync::Mutex;
use tokio_tungstenite::{connect_async, WebSocketStream};
use tower::ServiceBuilder;
use tracing::{debug, info};

// TODO: add configuration?
// const SERVER_ADDR: &str = "ws://192.168.101.194:7575";
const SERVER_ADDR: &str = "ws://127.0.0.1:7575";

type CodelabServer = SplitSink<
    WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
    tokio_tungstenite::tungstenite::Message,
>;
struct ServerState {
    client: ClientSocket,
    codelab_server: Arc<Mutex<CodelabServer>>,
    ignore_queue_recv: PeekableReceiver<DidChangeTextDocumentParams>,
    ignore_queue_send: Sender<DidChangeTextDocumentParams>,
    ignore_pool: Vec<DidChangeTextDocumentParams>,
}

impl LanguageServer for ServerState {
    type Error = ResponseError;
    type NotifyResult = ControlFlow<async_lsp::Result<()>>;

    fn initialize(
        &mut self,
        params: InitializeParams,
    ) -> BoxFuture<'static, Result<InitializeResult, Self::Error>> {
        info!("Initialized");
        debug!("Initialize params: {params:?}");
        Box::pin(async move {
            Ok(InitializeResult {
                capabilities: ServerCapabilities {
                    text_document_sync: Some(Kind(TextDocumentSyncKind::INCREMENTAL)),
                    ..ServerCapabilities::default()
                },
                server_info: None,
            })
        })
    }

    fn did_change_configuration(
        &mut self,
        _: DidChangeConfigurationParams,
    ) -> ControlFlow<async_lsp::Result<()>> {
        ControlFlow::Continue(())
    }

    fn did_open(&mut self, params: DidOpenTextDocumentParams) -> Self::NotifyResult {
        // TODO: open document for peers
        info!("opened document: {}", params.text_document.uri);
        ControlFlow::Continue(())
    }

    fn did_change(&mut self, params: DidChangeTextDocumentParams) -> Self::NotifyResult {
        while let Ok(ignore) = self.ignore_queue_recv.try_recv() {
            self.ignore_pool.push(ignore);
        }
        if let Some(i) = self
            .ignore_pool
            .iter()
            .enumerate()
            .find(|(_, change)| changes_eq(change, &params))
            .map(|(i, _)| i)
        // if self
        //     .ignore_queue_recv
        //     .try_recv_peek()
        //     .unwrap()
        //     .is_some_and(|change| changes_eq(change, &params))
        {
            self.ignore_pool.remove(i);
            // self.ignore_queue_recv.try_recv().unwrap();
            info!("Ignoring next change as it was received");
            return ControlFlow::Continue(());
        }
        let mutex = self.codelab_server.clone();
        tokio::spawn({
            async move {
                mutex
                    .lock()
                    .await
                    .send(tokio_tungstenite::tungstenite::Message::Text(
                        serde_json::to_string(&Message { change: params })
                            .expect("To be able to construct a json")
                            .into(),
                    ))
                    .await
                    .expect("Failed to send message to server");
            }
        });
        ControlFlow::Continue(())
    }
}

fn content_changes_eq(
    a: &TextDocumentContentChangeEvent,
    b: &TextDocumentContentChangeEvent,
) -> bool {
    a.range == b.range && a.text == b.text
}

fn changes_eq(a: &DidChangeTextDocumentParams, b: &DidChangeTextDocumentParams) -> bool {
    let eq = a.text_document.uri == b.text_document.uri
        && a.content_changes
            .iter()
            .zip(b.content_changes.iter())
            .all(|(a, b)| content_changes_eq(a, b));
    // if !eq {
    //     info!("{:#?} == {:#?}", &a.content_changes, &b.content_changes);
    // }
    eq
}

struct ChangeEvent(DidChangeTextDocumentParams);

impl ServerState {
    fn new_router(editor_client: ClientSocket, codelab_server: CodelabServer) -> Router<Self> {
        let (ignore_queue_send, ignore_queue_recv) = mpsc::channel();
        let ignore_queue_recv = PeekableReceiver::from(ignore_queue_recv);
        let mut router = Router::from_language_server(Self {
            client: editor_client,
            codelab_server: Arc::new(Mutex::new(codelab_server)),
            ignore_queue_recv,
            ignore_queue_send,
            ignore_pool: Vec::new(),
        });
        router.event(Self::on_change);
        router
    }

    fn on_change(&mut self, event: ChangeEvent) -> ControlFlow<async_lsp::Result<()>> {
        // we don't want to send what we just received otherwise we create an infinite loop between clients
        self.ignore_queue_send.send(event.0.clone()).unwrap();
        let _ = self
            .client
            .apply_edit(change_event_to_workspace_edit(&event.0));
        ControlFlow::Continue(())
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    let (ws, _) = connect_async(SERVER_ADDR)
        .await
        .context("Could not connect to server")?;
    let (send, mut recv) = ws.split();

    let (server, _) = async_lsp::MainLoop::new_server(|client| {
        tokio::spawn({
            let client = client.clone();
            async move {
                while let Some(msg) = recv
                    .try_next()
                    .await
                    .context("Failed to recv updates from server")?
                {
                    let msg: Message = serde_json::from_str(
                        msg.to_text().context("Server sent a non text message")?,
                    )
                    .context("Server sent an invalid message")?;
                    if client.emit(ChangeEvent(msg.change)).is_err() {
                        break;
                    }
                }
                Ok::<(), anyhow::Error>(())
            }
        });

        ServiceBuilder::new()
            .layer(TracingLayer::default())
            .layer(LifecycleLayer::default())
            .layer(CatchUnwindLayer::default())
            .layer(ConcurrencyLayer::default())
            .layer(ClientProcessMonitorLayer::new(client.clone()))
            .service(ServerState::new_router(client, send))
    });

    init_logger();

    // Prefer truly asynchronous piped stdin/stdout without blocking tasks.
    #[cfg(unix)]
    let (stdin, stdout) = (
        async_lsp::stdio::PipeStdin::lock_tokio().unwrap(),
        async_lsp::stdio::PipeStdout::lock_tokio().unwrap(),
    );
    // Fallback to spawn blocking read/write otherwise.
    #[cfg(not(unix))]
    let (stdin, stdout) = (
        tokio_util::compat::TokioAsyncReadCompatExt::compat(tokio::io::stdin()),
        tokio_util::compat::TokioAsyncWriteCompatExt::compat_write(tokio::io::stdout()),
    );

    match server.run_buffered(stdin, stdout).await {
        Ok(()) => Ok(()),
        Err(async_lsp::Error::Eof) => Ok(()),
        Err(err) => Err(anyhow!("Failed to run on stdio: {err:#?}")),
    }
}
