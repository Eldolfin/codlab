/// Runs the lsp-server (client bin) with a mocked lsp-client
/// and with randomized inputs
mod common;

use assert_cmd::cargo::CommandCargoExt as _;
use async_lsp::lsp_types::{
    DidChangeTextDocumentParams, Position, Range, TextDocumentContentChangeEvent, Url,
};
use codlab::{init_logger, logger};
use common::lsp_client;
use proptest::collection::vec;
use proptest::{prelude::Arbitrary, prop_compose, proptest, test_runner::TestRunner};
use proptest_derive::Arbitrary;
use std::{env::temp_dir, process::Command, time::Duration};

const MAX_POSITION_SIZE: u32 = 1;
const INPUT_REGEX: &str = "[a-z]{1,5}";

prop_compose! {
    fn arb_text_document_change()(
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

#[derive(Debug, Arbitrary)]
enum Client {
    Client1,
    Client2,
}

// single document change from one of the clients
#[derive(Debug, Arbitrary)]
struct ClientChange {
    from: Client,
    #[proptest(strategy = "vec(arb_text_document_change(), (1..10))")]
    changes: Vec<TextDocumentContentChangeEvent>,
}

#[derive(Debug, Arbitrary)]
struct TestCase {
    #[proptest(strategy = "vec(ClientChange::arbitrary(), (1..10))")]
    changes: Vec<ClientChange>,
}

async fn test_mocked_clients_quickcheck(params: TestCase) -> anyhow::Result<()> {
    let work_dir = temp_dir();
    let mut client1 = lsp_client::MockClient::new().await;
    let mut client2 = lsp_client::MockClient::new().await;

    let file_uri = Url::from_file_path(work_dir.join("src/lib.rs")).unwrap();

    for change in params.changes {
        match change.from {
            Client::Client1 => &mut client1,
            Client::Client2 => &mut client2,
        }
        .did_change(DidChangeTextDocumentParams {
            text_document: async_lsp::lsp_types::VersionedTextDocumentIdentifier {
                uri: file_uri.clone(),
                version: 0,
            },
            content_changes: change.changes,
        })
        .await?;
    }

    // this is not great
    tokio::time::sleep(Duration::from_millis(50)).await;

    assert!(!client1.document().is_empty());
    assert_eq!(client1.document(), client2.document());

    client1.drop().await;
    client2.drop().await;
    Ok(())
}

#[test]
fn test_mocked_clients_quickcheck_sync() -> proptest::test_runner::TestCaseResult {
    let mut _server_child =
        async_process::Command::from(Command::cargo_bin("server").expect("server binary to exist"))
            .kill_on_drop(true)
            .spawn()
            .expect("could not spawn server");
    let mut runner = TestRunner::default();
    logger::init("tests");
    let tokio_runtime = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .unwrap();

    runner
        .run(&TestCase::arbitrary(), |test_params| {
            tokio_runtime
                .block_on(async move { test_mocked_clients_quickcheck(test_params).await })
                .expect("test to pass");
            Ok(())
        })
        .unwrap();
    Ok(())
}
