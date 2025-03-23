use std::{collections::HashMap, sync::Arc};

use anyhow::Context;
use codlab::messages::Message;
use futures::{future::join_all, SinkExt, StreamExt, TryStreamExt as _};
use tokio::{net::TcpListener, sync::Mutex};
use tokio_tungstenite::tungstenite;
use tracing::{error, info, Level};
// TODO: config
const LISTEN_ADDR: &str = "0.0.0.0:7575";

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .with_ansi(false)
        .with_writer(std::io::stderr)
        .init();

    info!("Listening at ws://{LISTEN_ADDR}");
    let listener = TcpListener::bind(LISTEN_ADDR)
        .await
        .with_context(|| format!("Failed to bind at addr {LISTEN_ADDR}"))?;

    let clients = Arc::new(Mutex::new(HashMap::new()));

    while let Ok((stream, peer_addr)) = listener.accept().await {
        let peer_addr = peer_addr.to_string();
        info!("Client connected: {peer_addr}");
        let ws = match tokio_tungstenite::accept_async(stream).await {
            Ok(ws) => ws,
            Err(err) => {
                error!("Error in websocket handshake: {err:#}");
                break;
            }
        };
        let (send, mut recv) = ws.split();
        let clients = clients.clone();
        clients.lock().await.insert(peer_addr.clone(), send);
        tokio::spawn(async move {
            while let Ok(Some(msg)) = recv
                .try_next()
                .await
                .inspect_err(|err| error!("Failed to recv client messages: {err:#}"))
            {
                info!("received msg: {msg:#?}");
                let msg: Message =
                    serde_json::from_str(&msg.into_text().expect("Client sent a non text message"))
                        .expect("Client sent an invalid message");
                let mut lock = clients.lock().await;
                let futs =
                    lock.iter_mut()
                        .filter(|(addr, _)| addr != &&peer_addr)
                        .map(|(_, send)| {
                            send.send(tungstenite::Message::Text(
                                serde_json::to_string(&msg)
                                    .expect("To be able to construct a json")
                                    .into(),
                            ))
                        });
                join_all(futs).await;
            }
        });
    }
    Ok(())
}
