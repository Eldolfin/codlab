mod common;

use async_lsp::lsp_types::{DidChangeTextDocumentParams, Url, VersionedTextDocumentIdentifier};
use codlab::change::ChangeEvent;
use common::proptest_structs::arb_did_change_text_document_param;
use proptest::prelude::*;
use rstest::rstest;

proptest! {

    #[test]
    fn test_change_split_into_units_proptest(
        didchange in arb_did_change_text_document_param()
    ) {
        let unit_changes = ChangeEvent::new(didchange).split_into_units();

        for change in unit_changes {
            prop_assert_eq!(change.change.content_changes.len(), 1);
            if !change.change.content_changes[0].text.is_empty() {
                prop_assert_eq!(change.change.content_changes[0].range.unwrap().start, change.change.content_changes[0].range.unwrap().end);
            }
        }
    }
}
fn whatever_versioned_text_document() -> VersionedTextDocumentIdentifier {
    VersionedTextDocumentIdentifier {
        uri: Url::parse("file://test.md").unwrap(),
        version: 0,
    }
}
#[rstest]
#[case(
        ChangeEvent {
            change: DidChangeTextDocumentParams {
                text_document: whatever_versioned_text_document(),
                content_changes: vec![async_lsp::lsp_types::TextDocumentContentChangeEvent {
                    range: Some(async_lsp::lsp_types::Range {
                        start: async_lsp::lsp_types::Position::new(0, 0),
                        end: async_lsp::lsp_types::Position::new(0, 0),
                    }),
                    text: "abc".to_owned(),
                    range_length: None,
                }],
            },
        },
        vec![
            ChangeEvent {
                change: DidChangeTextDocumentParams {
                    text_document: whatever_versioned_text_document(),
                    content_changes: vec![async_lsp::lsp_types::TextDocumentContentChangeEvent {
                        range: Some(async_lsp::lsp_types::Range {
                            start: async_lsp::lsp_types::Position::new(0, 0),
                            end: async_lsp::lsp_types::Position::new(0, 0),
                        }),
                        text: "a".to_owned(),
                        range_length: None,
                    }],
                },
            },
            ChangeEvent {
                change: DidChangeTextDocumentParams {
                    text_document: whatever_versioned_text_document(),
                    content_changes: vec![async_lsp::lsp_types::TextDocumentContentChangeEvent {
                        range: Some(async_lsp::lsp_types::Range {
                            start: async_lsp::lsp_types::Position::new(0, 1),
                            end: async_lsp::lsp_types::Position::new(0, 1),
                        }),
                        text: "b".to_owned(),
                        range_length: None,
                    }],
                },
            },
            ChangeEvent {
                change: DidChangeTextDocumentParams {
                    text_document: whatever_versioned_text_document(),
                    content_changes: vec![async_lsp::lsp_types::TextDocumentContentChangeEvent {
                        range: Some(async_lsp::lsp_types::Range {
                            start: async_lsp::lsp_types::Position::new(0, 2),
                            end: async_lsp::lsp_types::Position::new(0, 2),
                        }),
                        text: "c".to_owned(),
                        range_length: None,
                    }],
                },
            },
        ],
    )]
#[case(
        ChangeEvent {
            change: DidChangeTextDocumentParams {
                text_document: whatever_versioned_text_document(),
                content_changes: vec![async_lsp::lsp_types::TextDocumentContentChangeEvent {
                    range: Some(async_lsp::lsp_types::Range {
                        start: async_lsp::lsp_types::Position::new(0, 0),
                        end: async_lsp::lsp_types::Position::new(0, 0),
                    }),
                    text: "a\nc".to_owned(),
                    range_length: None,
                }],
            },
        },
        vec![
            ChangeEvent {
                change: DidChangeTextDocumentParams {
                    text_document: whatever_versioned_text_document(),
                    content_changes: vec![async_lsp::lsp_types::TextDocumentContentChangeEvent {
                        range: Some(async_lsp::lsp_types::Range {
                            start: async_lsp::lsp_types::Position::new(0, 0),
                            end: async_lsp::lsp_types::Position::new(0, 0),
                        }),
                        text: "a".to_owned(),
                        range_length: None,
                    }],
                },
            },
            ChangeEvent {
                change: DidChangeTextDocumentParams {
                    text_document: whatever_versioned_text_document(),
                    content_changes: vec![async_lsp::lsp_types::TextDocumentContentChangeEvent {
                        range: Some(async_lsp::lsp_types::Range {
                            start: async_lsp::lsp_types::Position::new(0, 1),
                            end: async_lsp::lsp_types::Position::new(0, 1),
                        }),
                        text: "\n".to_owned(),
                        range_length: None,
                    }],
                },
            },
            ChangeEvent {
                change: DidChangeTextDocumentParams {
                    text_document: whatever_versioned_text_document(),
                    content_changes: vec![async_lsp::lsp_types::TextDocumentContentChangeEvent {
                        range: Some(async_lsp::lsp_types::Range {
                            start: async_lsp::lsp_types::Position::new(1, 0),
                            end: async_lsp::lsp_types::Position::new(1, 0),
                        }),
                        text: "c".to_owned(),
                        range_length: None,
                    }],
                },
            },
        ],
    )]
fn test_change_split_into_units(
    #[case] change: ChangeEvent,
    #[case] expected_units: Vec<ChangeEvent>,
) {
    use pretty_assertions::assert_eq;
    assert_eq!(change.split_into_units(), expected_units);
}
