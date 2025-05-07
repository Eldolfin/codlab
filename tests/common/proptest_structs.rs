use async_lsp::lsp_types::{
    DidChangeTextDocumentParams, Position, Range, TextDocumentContentChangeEvent, Url,
    VersionedTextDocumentIdentifier,
};
use proptest::{prelude::prop, prop_compose};

const INPUT_REGEX: &str = "[a-z]{1,5}";
const MAX_POSITION_SIZE: u32 = 10;

prop_compose! {
    pub fn arb_text_document_change()(
        input      in INPUT_REGEX,
        start_line in 0..MAX_POSITION_SIZE,
        start_char in 0..MAX_POSITION_SIZE,
        end_line   in 0..MAX_POSITION_SIZE,
        end_char   in 0..MAX_POSITION_SIZE
    ) -> TextDocumentContentChangeEvent {
        TextDocumentContentChangeEvent {
            range: Some(Range {
                start: Position {
                    line: start_line,
                    character:start_char
                },
                end: Position { line:
                    end_line, character:
                    end_char
                }
            }),
            range_length: None,
            text: input,
        }
    }
}
prop_compose! {
    pub fn arb_did_change_text_document_param()(
        content_changes in prop::collection::vec(arb_text_document_change(), 1..10),
    ) -> DidChangeTextDocumentParams {
        DidChangeTextDocumentParams {
            text_document: VersionedTextDocumentIdentifier {
                uri: Url::parse("file://test.md").unwrap(),
                version: 0,
            },
            content_changes,
        }

    }
}
