use std::collections::HashMap;

use async_lsp::lsp_types::{
    ApplyWorkspaceEditParams, DidChangeTextDocumentParams, TextEdit, WorkspaceEdit,
};

pub mod messages;
pub mod peekable_channel;

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
                    .map(|change| TextEdit {
                        range: change.range.expect("Changes to have a range"),
                        new_text: change.text.clone(),
                    })
                    .collect(),
            )])),
            ..Default::default()
        },
    }
}
