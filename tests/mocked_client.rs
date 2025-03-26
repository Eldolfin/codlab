mod common;

use assert_cmd::cargo::CommandCargoExt as _;
use async_lsp::lsp_types::{
    DidChangeTextDocumentParams, DidOpenTextDocumentParams, Position, Range,
    TextDocumentContentChangeEvent, TextDocumentItem, Url,
};
use common::lsp_client;
use std::{env::temp_dir, process::Command, time::Duration};
use tracing::Level;

/// Runs the lsp-server (client bin) with a mocked lsp-client

#[tokio::test]
async fn test_mocked_clients() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .pretty()
        .with_writer(std::io::stderr)
        .init();
    // TODO:
    // - run server binary
    // - run client binary using async-lsp's client
    // - emulate an editor
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

    client1
        .did_change(DidChangeTextDocumentParams {
            text_document: async_lsp::lsp_types::VersionedTextDocumentIdentifier {
                uri: file_uri.clone(),
                version: 0,
            },
            content_changes: vec![TextDocumentContentChangeEvent {
                range: Some(Range::new(Position::new(0, 0), Position::new(0, 0))),
                text: added.to_owned(),
                range_length: None,
            }],
        })
        .await?;

    tokio::time::sleep(Duration::from_millis(20)).await;

    let expected = format!("{}{}", added, text);
    assert_eq!(client1.document(), expected);
    assert_eq!(client2.document(), expected);

    client1.drop().await;
    client2.drop().await;
    Ok(())
}
