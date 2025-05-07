pub mod change;
pub mod common;
pub mod messages;
pub mod peekable_channel;

use std::collections::HashMap;

use async_lsp::lsp_types::{
    ApplyWorkspaceEditParams, DidChangeTextDocumentParams, Position, Range, TextEdit, WorkspaceEdit,
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
                    .map(|change| {
                        // if there is no range, just replace the whole document
                        let range = change.range.unwrap_or(Range::new(
                            Position::new(0, 0),
                            // some editors might not like this
                            Position::new(u32::MAX, u32::MAX),
                        ));
                        TextEdit {
                            range,
                            new_text: change.text.clone(),
                        }
                    })
                    .collect(),
            )])),
            ..Default::default()
        },
    }
}
