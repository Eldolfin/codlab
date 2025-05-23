use anyhow::{Context, anyhow};
use async_lsp::{
    ClientSocket, LanguageClient, LanguageServer, ResponseError,
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
};
use clap::Parser;
use codlab::{
    change::{self, ChangeEvent},
    change_event_to_workspace_edit,
    common::init_logger,
    messages::{Change, ClientMessage, CommonMessage, ServerMessage},
    peekable_channel::PeekableReceiver,
};
use futures::{SinkExt, StreamExt as _, TryStreamExt, future::BoxFuture, stream::SplitSink};
use std::{
    ops::ControlFlow,
    sync::{
        Arc,
        mpsc::{self, Sender},
    },
};
use tokio::sync::Mutex;
use tokio_tungstenite::{WebSocketStream, connect_async};
use tower::ServiceBuilder;
use tracing::{debug, info};
use uuid::Uuid;

type CodelabServer = SplitSink<
    WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
    tokio_tungstenite::tungstenite::Message,
>;
struct ServerState {
    #[allow(dead_code)]
    client: ClientSocket,
    codelab_server: Arc<Mutex<CodelabServer>>,
    ignore_queue_recv: PeekableReceiver<Vec<change::ChangeEvent>>,
    ignore_queue_send: Sender<Vec<change::ChangeEvent>>,
    ignore_pool: Vec<change::ChangeEvent>,
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
                    text_document_sync: Some(Kind(TextDocumentSyncKind::FULL)),
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
        tokio::spawn({
            let send = self.codelab_server.clone();
            async move {
                client_send_msg(
                    &send,
                    &ClientMessage::Common(CommonMessage::Change(Change {
                        change: params,
                        id: Uuid::new_v4(),
                    })),
                )
                .await;
            }
        });
        ControlFlow::Continue(())
    }
}

async fn client_send_msg(send: &Arc<Mutex<CodelabServer>>, msg: &ClientMessage) {
    send.lock()
        .await
        .send(tokio_tungstenite::tungstenite::Message::Text(
            serde_json::to_string(msg)
                .expect("To be able to construct a json")
                .into(),
        ))
        .await
        .expect("Failed to send message to server");
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

impl ServerState {
    fn new_router(
        editor_client: ClientSocket,
        codelab_server: Arc<Mutex<CodelabServer>>,
    ) -> Router<Self> {
        let (ignore_queue_send, ignore_queue_recv) = mpsc::channel();
        let ignore_queue_recv = PeekableReceiver::from(ignore_queue_recv);
        let mut router = Router::from_language_server(Self {
            client: editor_client,
            codelab_server,
            ignore_queue_recv,
            ignore_queue_send,
            ignore_pool: Vec::new(),
        });
        router.event(Self::on_change);
        router
    }

    fn on_change(&mut self, event: change::ChangeEvent) -> ControlFlow<async_lsp::Result<()>> {
        // we don't want to send what we just received otherwise we create an infinite loop between clients
        // self.ignore_queue_send.send(()).unwrap();
        ControlFlow::Continue(())
    }
}

#[derive(Parser)]
struct Args {
    server_addr: String,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let (ws, _) = connect_async(args.server_addr)
        .await
        .context("Could not connect to server")?;
    let (send, mut recv) = ws.split();
    let send = Arc::new(Mutex::new(send));

    let (server, _) = async_lsp::MainLoop::new_server(|client| {
        tokio::spawn({
            let mut client = client.clone();
            let _send = send.clone();
            async move {
                while let Some(msg) = recv
                    .try_next()
                    .await
                    .context("Failed to recv updates from server")?
                {
                    let msg: ServerMessage = serde_json::from_str(
                        msg.to_text().context("Server sent a non text message")?,
                    )
                    .context("Server sent an invalid message")?;
                    match msg {
                        ServerMessage::Common(common_message) => match common_message {
                            CommonMessage::Change(change) => {
                                if client
                                    .emit(change::ChangeEvent::new(change.change.clone()))
                                    .is_err()
                                {
                                    break;
                                }
                                client
                                    .apply_edit(change_event_to_workspace_edit(&change.change))
                                    .await
                                    .unwrap();
                                debug!("client: applied remote edit successfully!");
                            }
                        },
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
