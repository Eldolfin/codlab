use anyhow::Context;
use async_lsp::client_monitor::ClientProcessMonitorLayer;
use async_lsp::concurrency::ConcurrencyLayer;
use async_lsp::lsp_types::TextDocumentSyncCapability::Kind;
use async_lsp::lsp_types::{
    ApplyWorkspaceEditParams, DidChangeConfigurationParams, DidChangeTextDocumentParams,
    DidOpenTextDocumentParams, InitializeParams, InitializeResult, ServerCapabilities,
    TextDocumentSyncKind, TextEdit, WorkspaceEdit,
};
use async_lsp::panic::CatchUnwindLayer;
use async_lsp::router::Router;
use async_lsp::server::LifecycleLayer;
use async_lsp::tracing::TracingLayer;
use async_lsp::{ClientSocket, LanguageClient as _, LanguageServer, ResponseError};
use codlab::messages::Message;
use futures::future::BoxFuture;
use futures::stream::SplitSink;
use futures::{SinkExt, StreamExt as _, TryStreamExt};
use std::collections::HashMap;
use std::iter::Peekable;
use std::ops::ControlFlow;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_tungstenite::{connect_async, WebSocketStream};
use tower::ServiceBuilder;
use tracing::{info, Level};

// TODO: add configuration?
const SERVER_ADDR: &str = "ws://192.168.101.194:7575";

type CodelabServer = SplitSink<
    WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
    tokio_tungstenite::tungstenite::Message,
>;
struct ServerState {
    client: ClientSocket,
    codelab_server: Arc<Mutex<CodelabServer>>,
    ignore_queue_recv: Receiver<DidChangeTextDocumentParams>,
    ignore_queue_send: Sender<DidChangeTextDocumentParams>,
}

impl LanguageServer for ServerState {
    type Error = ResponseError;
    type NotifyResult = ControlFlow<async_lsp::Result<()>>;

    fn initialize(
        &mut self,
        params: InitializeParams,
    ) -> BoxFuture<'static, Result<InitializeResult, Self::Error>> {
        eprintln!("Initialize with {params:?}");
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
        info!("opened document: {:#?}", params.text_document);
        ControlFlow::Continue(())
    }

    fn did_change(&mut self, params: DidChangeTextDocumentParams) -> Self::NotifyResult {
        // info!("did_change: {:#?}", params);
        // FIXME: race condition here, if we change the doc before the request has time to be applied and the event fired from the editor, we don't send the change
        // TODO: use a peekable mpsc or something
        // if self
        //     .ignore_queue_recv
        //     .peek()
        //     .is_some_and(|change| change == &params)
        // {
        // self.ignore_queue_recv.next();
        if self.ignore_queue_recv.try_recv().is_ok() {
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
                info!("sent message ez");
            }
        });
        ControlFlow::Continue(())
    }
}

struct ChangeEvent(DidChangeTextDocumentParams);

impl ServerState {
    fn new_router(editor_client: ClientSocket, codelab_server: CodelabServer) -> Router<Self> {
        let (ignore_queue_send, ignore_queue_recv) = mpsc::channel();
        // let ignore_queue_recv = ignore_queue_recv.into_iter().peekable();
        let mut router = Router::from_language_server(Self {
            client: editor_client,
            codelab_server: Arc::new(Mutex::new(codelab_server)),
            ignore_queue_recv,
            ignore_queue_send,
        });
        router.event(Self::on_change);
        router
    }

    fn on_change(&mut self, event: ChangeEvent) -> ControlFlow<async_lsp::Result<()>> {
        // we don't want to send what we just received otherwise we create an infinite loop between clients
        self.ignore_queue_send.send(event.0.clone()).unwrap();
        let _ = self.client.apply_edit(ApplyWorkspaceEditParams {
            label: Some("TODO: edit labels".to_owned()),
            edit: WorkspaceEdit {
                changes: Some(HashMap::from([(
                    event.0.text_document.uri,
                    event
                        .0
                        .content_changes
                        .iter()
                        .map(|change| TextEdit {
                            range: change.range.expect("Changes to have a range"),
                            new_text: change.text.clone(),
                        })
                        .collect(),
                )])),
                ..Default::default()
            },
        });
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

    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .with_ansi(false)
        .with_writer(std::io::stderr)
        .init();

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

    server.run_buffered(stdin, stdout).await.unwrap();
    Ok(())
}
