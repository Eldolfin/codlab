use async_lsp::lsp_types::DidChangeTextDocumentParams;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Message {
    pub change: DidChangeTextDocumentParams,
}
