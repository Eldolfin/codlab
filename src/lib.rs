pub mod logger;
pub mod messages;
pub mod peekable_channel;
#[cfg(feature = "telemetry")]
pub mod telemetry;

use std::collections::HashMap;

use async_lsp::lsp_types::{
    ApplyWorkspaceEditParams, DidChangeTextDocumentParams, TextEdit, WorkspaceEdit,
};
use tracing::warn;

// TODO: move this somewhere else
pub fn change_event_to_workspace_edit(
    event: &DidChangeTextDocumentParams,
) -> ApplyWorkspaceEditParams {
    ApplyWorkspaceEditParams {
        // TODO: username in edit label
        label: Some("remote editor".to_owned()),
        edit: WorkspaceEdit {
            changes: Some(HashMap::from([(
                event.text_document.uri.clone(),
                event
                    .content_changes
                    .iter()
                    .filter_map(|change| {
                        if change.range.is_none() {
                            warn!("Skipping change which has no range: {:?}", change);
                        }
                        Some(TextEdit {
                            range: change.range?,
                            new_text: change.text.clone(),
                        })
                    })
                    .collect(),
            )])),
            ..Default::default()
        },
    }
}
