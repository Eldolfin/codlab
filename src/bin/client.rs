use anyhow::Context;
use async_lsp::client_monitor::ClientProcessMonitorLayer;
use async_lsp::concurrency::ConcurrencyLayer;
use async_lsp::lsp_types::TextDocumentSyncCapability::Kind;
use async_lsp::lsp_types::{
    DidChangeConfigurationParams, DidChangeTextDocumentParams, DidOpenTextDocumentParams,
    InitializeParams, InitializeResult, ServerCapabilities, TextDocumentSyncKind,
};
use async_lsp::panic::CatchUnwindLayer;
use async_lsp::router::Router;
use async_lsp::server::LifecycleLayer;
use async_lsp::tracing::TracingLayer;
use async_lsp::{ClientSocket, LanguageServer, ResponseError};
use codlab::messages::Message;
use futures::future::BoxFuture;
use futures::SinkExt;
use std::ops::ControlFlow;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio_tungstenite::{connect_async, WebSocketStream};
use tower::ServiceBuilder;
use tracing::{info, Level};

// TODO: add configuration?
const SERVER_ADDR: &str = "ws://127.0.0.1:7575";

type CodelabServer = WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;
struct ServerState {
    client: ClientSocket,
    counter: i32,
    codelab_server: Arc<Mutex<CodelabServer>>,
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
        params: DidChangeConfigurationParams,
    ) -> ControlFlow<async_lsp::Result<()>> {
        ControlFlow::Continue(())
    }

    fn did_open(&mut self, params: DidOpenTextDocumentParams) -> Self::NotifyResult {
        // TODO: open document for peers
        info!("opened document: {:#?}", params.text_document);
        ControlFlow::Continue(())
    }

    fn did_change(&mut self, params: DidChangeTextDocumentParams) -> Self::NotifyResult {
        // TODO: send change to peers
        info!("did_change: {:#?}", params);
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
            }
        });
        ControlFlow::Continue(())
    }
}

struct TickEvent;

impl ServerState {
    fn new_router(editor_client: ClientSocket, codelab_server: CodelabServer) -> Router<Self> {
        let mut router = Router::from_language_server(Self {
            client: editor_client,
            counter: 0,
            codelab_server: Arc::new(Mutex::new(codelab_server)),
        });
        router.event(Self::on_tick);
        router
    }

    fn on_tick(&mut self, _: TickEvent) -> ControlFlow<async_lsp::Result<()>> {
        info!("tick");
        self.counter += 1;
        // let _ = self.client.apply_edit(ApplyWorkspaceEditParams {
        //     label: Some("TODO: edit labels".to_owned()),
        //     edit: WorkspaceEdit {
        //         changes: Some(HashMap::from([(
        //             Url::parse("file:///home/oscar/Prog/Probe/codlab/test.md")
        //                 .expect("test url to be valid"),
        //             vec![TextEdit {
        //                 range: Range {
        //                     start: Position {
        //                         line: 0,
        //                         character: 0,
        //                     },
        //                     end: Position {
        //                         line: 0,
        //                         character: (self.counter - 1).to_string().len() as u32,
        //                     },
        //                 },
        //                 new_text: self.counter.to_string(),
        //             }],
        //         )])),
        //         ..Default::default()
        //     },
        // });
        ControlFlow::Continue(())
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    let (ws, _) = connect_async(SERVER_ADDR)
        .await
        .context("Could not connect to server")?;

    let (server, _) = async_lsp::MainLoop::new_server(|client| {
        tokio::spawn({
            let client = client.clone();
            async move {
                let mut interval = tokio::time::interval(Duration::from_secs(1));
                loop {
                    interval.tick().await;
                    if client.emit(TickEvent).is_err() {
                        break;
                    }
                }
            }
        });

        ServiceBuilder::new()
            .layer(TracingLayer::default())
            .layer(LifecycleLayer::default())
            .layer(CatchUnwindLayer::default())
            .layer(ConcurrencyLayer::default())
            .layer(ClientProcessMonitorLayer::new(client.clone()))
            .service(ServerState::new_router(client, ws))
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
