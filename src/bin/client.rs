use anyhow::{anyhow, Context};
use async_lsp::{
    client_monitor::ClientProcessMonitorLayer,
    concurrency::ConcurrencyLayer,
    lsp_types::{
        DidChangeConfigurationParams, DidChangeTextDocumentParams, DidOpenTextDocumentParams,
        InitializeParams, InitializeResult, ServerCapabilities, TextDocumentContentChangeEvent,
        TextDocumentSyncCapability::Kind, TextDocumentSyncKind, Url,
    },
    panic::CatchUnwindLayer,
    router::Router,
    server::LifecycleLayer,
    tracing::TracingLayer,
    ClientSocket, LanguageClient, LanguageServer, ResponseError,
};
use codlab::{
    change_event_to_workspace_edit, logger,
    messages::{Change, ClientMessage, CommonMessage, ServerMessage},
    peekable_channel::PeekableReceiver,
};
use futures::{future::BoxFuture, stream::SplitSink, SinkExt, StreamExt as _, TryStreamExt};
use opentelemetry::{
    global,
    trace::{TraceContextExt as _, Tracer},
    KeyValue,
};
use opentelemetry::{global::ObjectSafeSpan, propagation::TextMapPropagator, trace::Span};
use opentelemetry_sdk::propagation::TraceContextPropagator;
use std::{
    collections::HashMap,
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
use uuid::Uuid;

// TODO: add configuration?
// const SERVER_ADDR: &str = "ws://192.168.101.194:7575";
const SERVER_ADDR: &str = "ws://127.0.0.1:7575";

type CodelabServer = SplitSink<
    WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
    tokio_tungstenite::tungstenite::Message,
>;

#[derive(PartialEq, Eq)]
struct UnitChange {
    pub text_document: Url,
    pub change: TextDocumentContentChangeEvent,
}

struct LSPServerState {
    client: ClientSocket,
    codelab_server: Arc<Mutex<CodelabServer>>,
    ignore_queue_recv: PeekableReceiver<UnitChange>,
    ignore_queue_send: Sender<UnitChange>,
    ignore_pool: Vec<UnitChange>,
}

impl LanguageServer for LSPServerState {
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
        // forget old enough changes, assume the editor didn't respond for some reason
        let canceled_indices: Vec<_> = params
            .content_changes
            .iter()
            .enumerate()
            .filter(|(_, a)| {
                if self
                    .ignore_queue_recv
                    .try_recv_peek()
                    .unwrap()
                    .is_some_and(|b| {
                        params.text_document.uri == b.text_document
                            && content_changes_eq(a, &b.change)
                    })
                {
                    self.ignore_queue_recv.try_recv().unwrap();
                    true
                } else {
                    false
                }
            })
            .map(|(i, _)| i)
            .collect();
        let filtered = {
            let mut res = params.clone();
            for i in canceled_indices.iter().rev() {
                res.content_changes.remove(*i);
            }
            res
        };
        if filtered.content_changes.is_empty() {
            debug!("Canceled entire message");
        } else {
            global::tracer("LSPServerState").in_span("editor: DidChangeTextDocument", |cx| {
                tokio::spawn({
                    let send = self.codelab_server.clone();

                    let propagator = TraceContextPropagator::new();
                    let mut trace_context = HashMap::new();
                    propagator.inject_context(&cx, &mut trace_context);
                    let change = Change {
                        id: Uuid::new_v4(),
                        change: filtered,
                        trace_context,
                    };
                    async move {
                        let span = cx.span();
                        span.add_event(
                            "New change from editor",
                            vec![KeyValue::new("change_id", change.id.to_string())],
                        );
                        client_send_msg(
                            &send,
                            &ClientMessage::Common(CommonMessage::Change(change)),
                        )
                        .await;
                    }
                });
            });
        }
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

impl LSPServerState {
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

    fn on_change(
        &mut self,
        event: DidChangeTextDocumentParams,
    ) -> ControlFlow<async_lsp::Result<()>> {
        // we don't want to send what we just received otherwise we create an infinite loop between clients
        for sub_change in event.content_changes {
            self.ignore_queue_send
                .send(UnitChange {
                    text_document: event.text_document.uri.clone(),
                    change: sub_change,
                })
                .unwrap();
        }
        ControlFlow::Continue(())
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    #[cfg(feature = "telemetry")]
    let telemetry_providers = codlab::telemetry::init("codlab-client");
    #[cfg(not(feature = "telemetry"))]
    logger::init();

    // TODO: retries (usefull for development)
    // TODO: connecting to the server should be done after handshaking the lsp-client
    let (ws, _) = connect_async(SERVER_ADDR)
        .await
        .context("Could not connect to server")?;
    let (send, mut recv) = ws.split();
    let send = Arc::new(Mutex::new(send));

    let (server, _) = async_lsp::MainLoop::new_server(|client| {
        tokio::spawn({
            let mut client = client.clone();
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
                                let extracted_ctx =
                                    TraceContextPropagator::new().extract(&change.trace_context);
                                let mut span = global::tracer("server")
                                    .start_with_context("handle_change", &extracted_ctx);
                                span.add_event("Apply remote change", vec![]);
                                if client.emit(change.change.clone()).is_err() {
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
            .service(LSPServerState::new_router(client, send))
    });

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

    let res = match server.run_buffered(stdin, stdout).await {
        Ok(()) => Ok(()),
        Err(async_lsp::Error::Eof) => Ok(()),
        Err(err) => Err(anyhow!("Failed to run on stdio: {err:#?}")),
    };

    res.and(telemetry_providers.shutdown())
}
