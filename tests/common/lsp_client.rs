use std::ops::ControlFlow;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};

use assert_cmd::cargo::CommandCargoExt as _;
use async_lsp::concurrency::ConcurrencyLayer;
use async_lsp::lsp_types::request::ApplyWorkspaceEdit;
use async_lsp::lsp_types::{
    ApplyWorkspaceEditParams, ApplyWorkspaceEditResponse, ClientCapabilities,
    DidChangeTextDocumentParams, DidOpenTextDocumentParams, InitializeParams, InitializedParams,
    PublishDiagnosticsParams, ShowMessageParams, WindowClientCapabilities,
};
use async_lsp::panic::CatchUnwindLayer;
use async_lsp::router::Router;
use async_lsp::tracing::TracingLayer;
use async_lsp::{LanguageClient, LanguageServer, ResponseError, ServerSocket};
use async_process::Child;
use codlab::change_event_to_workspace_edit;
use tokio::task::JoinHandle;
use tower::ServiceBuilder;
use tracing::info;

struct ClientState {
    document: Arc<Mutex<Vec<String>>>,
}

impl LanguageClient for ClientState {
    type Error = ResponseError;
    type NotifyResult = ControlFlow<async_lsp::Result<()>>;

    fn publish_diagnostics(&mut self, _: PublishDiagnosticsParams) -> Self::NotifyResult {
        ControlFlow::Continue(())
    }

    fn show_message(&mut self, params: ShowMessageParams) -> Self::NotifyResult {
        tracing::info!("Message {:?}: {}", params.typ, params.message);
        ControlFlow::Continue(())
    }
}

impl ClientState {
    fn new_router(document: Arc<Mutex<Vec<String>>>) -> Router<Self> {
        let mut router = Router::from_language_client(ClientState { document });
        router.event(Self::on_stop);
        router.event(Self::on_local_change);
        router.request::<ApplyWorkspaceEdit, _>(|state, params| {
            info!("Received apply edit: {params:#?}");
            state.apply_edits_impl(params);
            async move {
                Ok(ApplyWorkspaceEditResponse {
                    applied: true,
                    failure_reason: None,
                    failed_change: None,
                })
            }
        });
        router
    }

    fn on_stop(&mut self, _: Stop) -> ControlFlow<async_lsp::Result<()>> {
        ControlFlow::Break(Ok(()))
    }

    fn on_local_change(
        &mut self,
        edit: ApplyWorkspaceEditParams,
    ) -> ControlFlow<async_lsp::Result<()>> {
        info!("Applying local change");
        self.apply_edits_impl(edit);
        ControlFlow::Continue(())
    }

    fn apply_edits_impl(&mut self, params: async_lsp::lsp_types::ApplyWorkspaceEditParams) {
        for change in params
            .edit
            .changes
            .expect("edit.changes to be present")
            .values()
            .flatten()
        {
            let mut document = self.document.lock().unwrap();
            let position = change.range.start;
            if let Some(line) = document.get_mut(position.line as usize) {
                line.insert_str(position.character as usize, &change.new_text);
            } else {
                // add empty lines up to the change
                for _ in 0..position.line as usize - document.len() {
                    document.push(String::new());
                }
                document.push(change.new_text.clone());
            }
        }
    }
}

struct Stop;

pub struct MockClient {
    pub server: ServerSocket,
    /// lines of the edited file
    // TODO: support multi documents
    document: Arc<Mutex<Vec<String>>>,
    mainloop_fut: JoinHandle<()>,
    _child: Child,
}

impl MockClient {
    pub async fn new() -> Self {
        let document = Arc::new(Mutex::new(vec![]));
        let (mainloop, mut server) = async_lsp::MainLoop::new_client(|_server| {
            ServiceBuilder::new()
                .layer(TracingLayer::default())
                .layer(CatchUnwindLayer::default())
                .layer(ConcurrencyLayer::default())
                .service(ClientState::new_router(document.clone()))
        });

        let mut child = async_process::Command::from(
            Command::cargo_bin("client").expect("client binary to exist"),
        )
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .kill_on_drop(true)
        .spawn()
        .expect("Failed run rust-analyzer");

        let stdout = child.stdout.take().unwrap();
        let stdin = child.stdin.take().unwrap();
        let mainloop_fut = tokio::spawn(async move {
            mainloop.run_buffered(stdout, stdin).await.unwrap();
        });

        // Initialize.
        let init_ret = server
            .initialize(InitializeParams {
                capabilities: ClientCapabilities {
                    window: Some(WindowClientCapabilities {
                        work_done_progress: Some(true),
                        ..WindowClientCapabilities::default()
                    }),
                    ..ClientCapabilities::default()
                },
                ..InitializeParams::default()
            })
            .await
            .unwrap();
        info!("Initialized: {init_ret:?}");
        server.initialized(InitializedParams {}).unwrap();

        Self {
            server,
            mainloop_fut,
            _child: child,
            document,
        }
    }

    pub fn did_open(&mut self, params: DidOpenTextDocumentParams) -> async_lsp::Result<()> {
        *self.document.lock().unwrap() = params
            .text_document
            .text
            .lines()
            .map(|s| s.to_owned())
            .collect();
        self.server.did_open(params)
    }

    pub async fn did_change(
        &mut self,
        params: DidChangeTextDocumentParams,
    ) -> async_lsp::Result<()> {
        self.server
            .emit(change_event_to_workspace_edit(&params))
            .expect("Can apply local changes");
        self.server.did_change(params)
    }

    pub fn document(&self) -> String {
        self.document.lock().unwrap().join("\n")
    }

    // manual drop because Async drop doesn't exist yet
    pub async fn drop(mut self) {
        // Shutdown.
        self.server.shutdown(()).await.unwrap();
        self.server.exit(()).unwrap();

        self.server.emit(Stop).unwrap();
        self.mainloop_fut.await.unwrap();
    }
}
