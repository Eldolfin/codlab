use async_lsp::lsp_types::{
    DidChangeTextDocumentParams, Position, Range, TextDocumentContentChangeEvent,
};

#[derive(Debug, PartialEq, Eq)]
pub struct ChangeEvent {
    pub change: DidChangeTextDocumentParams,
}

impl ChangeEvent {
    pub fn new(change: DidChangeTextDocumentParams) -> Self {
        Self { change }
    }

    /// Split a change into unit changes, that is:
    /// ```txt
    ///   change.content_changes.len() == 1
    ///   &&
    ///   (
    ///     change.content_changes[0].text.len() == 0
    ///     ||
    ///     change.content_changes[0].text.len() == 1 && change.content_changes[0].range(start == end)
    ///   )
    /// ```
    pub fn split_into_units(&self) -> Vec<Self> {
        let mut unit_changes = vec![];
        for change in &self.change.content_changes {
            let mut pos = Position {
                line: change.range.unwrap().start.line,
                character: change.range.unwrap().start.character,
            };
            for char in change.text.chars() {
                let range = Range {
                    start: pos,
                    end: pos,
                };
                unit_changes.push(Self::new(DidChangeTextDocumentParams {
                    text_document: self.change.text_document.clone(),
                    content_changes: vec![TextDocumentContentChangeEvent {
                        range: Some(range),
                        range_length: None,
                        text: char.to_string(),
                    }],
                }));
                match char {
                    '\n' => {
                        pos.character = 0;
                        pos.line += 1;
                    }
                    _ => pos.character += 1,
                };
            }
        }
        unit_changes
    }
}
