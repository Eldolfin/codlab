use async_lsp::lsp_types::DidChangeTextDocumentParams;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct Change {
    pub id: Uuid,
    pub change: DidChangeTextDocumentParams,
    pub trace_context: std::collections::HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum CommonMessage {
    Change(Change),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ClientMessage {
    /// Confirms that a change was applied
    AcknowledgeChange(Uuid),
    Common(CommonMessage),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ServerMessage {
    Common(CommonMessage),
}
