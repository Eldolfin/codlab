/// Runs the lsp-server (client bin) with a mocked lsp-client
/// and with randomized inputs
mod common;

use assert_cmd::cargo::CommandCargoExt as _;
use async_lsp::lsp_types::{
    DidChangeTextDocumentParams, DidOpenTextDocumentParams, Position, Range,
    TextDocumentContentChangeEvent, TextDocumentItem, Url,
};
use common::lsp_client;
use proptest::{prelude::Arbitrary, proptest};
use proptest_derive::Arbitrary;
use std::{env::temp_dir, process::Command, time::Duration};
use tracing::Level;

#[derive(Debug)]
struct TestTextDocumentContentChangeEvent(TextDocumentContentChangeEvent);

impl Arbitrary for TestTextDocumentContentChangeEvent {
    type Parameters = (String);
    type Strategy = ();

    fn arbitrary_with(args: Self::Parameters) -> Self::Strategy {
        todo!()
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
    change: TestTextDocumentContentChangeEvent,
}

#[derive(Debug, Arbitrary)]
struct TestCase {
    changes: Vec<ClientChange>,
}

async fn test_mocked_clients_quickcheck(params: TestCase) -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .pretty()
        .with_writer(std::io::stderr)
        .init();

    let mut _server_child =
        async_process::Command::from(Command::cargo_bin("server").expect("server binary to exist"))
            .kill_on_drop(true)
            .spawn()
            .expect("could not spawn server");

    let work_dir = temp_dir();
    let mut client1 = lsp_client::MockClient::new().await;
    let client2 = lsp_client::MockClient::new().await;

    let file_uri = Url::from_file_path(work_dir.join("src/lib.rs")).unwrap();
    let text = ""; // TODO: send open document to others for initial content
    let added = "test";
    client1.did_open(DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: file_uri.clone(),
            language_id: "rust".into(),
            version: 0,
            text: text.into(),
        },
    })?;

    for change in params.changes {
        // client1
        //     .did_change(DidChangeTextDocumentParams {
        //         text_document: async_lsp::lsp_types::VersionedTextDocumentIdentifier {
        //             uri: file_uri.clone(),
        //             version: 0,
        //         },
        //         content_changes: vec![TextDocumentContentChangeEvent {
        //             range: Some(Range::new(Position::new(0, 0), Position::new(0, 0))),
        //             text: added.to_owned(),
        //             range_length: None,
        //         }],
        //     })
        //     .await?;
        todo!()
    }

    // this is not great
    tokio::time::sleep(Duration::from_millis(10)).await;

    // let expected = format!("{}{}", added, text);
    // assert_eq!(client1.document(), expected);
    // assert_eq!(client2.document(), expected);
    assert!(!client1.document().is_empty());
    assert_eq!(client1.document(), client2.document());

    client1.drop().await;
    client2.drop().await;
    Ok(())
}

proptest! {
    #[test]
    fn test_mocked_clients_quickcheck_sync(params: TestCase) {dbg!(client1_changes);
    }
}
