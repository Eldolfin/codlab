use std::{collections::HashMap, sync::Arc};

use anyhow::Context;
use codlab::{
    common::init_logger,
    messages::{ClientMessage, ServerMessage},
};
use futures::{SinkExt, StreamExt, TryStreamExt as _, future::join_all, stream::SplitSink};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::Mutex,
};
use tokio_tungstenite::{WebSocketStream, tungstenite};
use tracing::{debug, error, info};
// TODO: config
const LISTEN_ADDR: &str = "0.0.0.0:7575";

struct Client {
    send: SplitSink<WebSocketStream<TcpStream>, tokio_tungstenite::tungstenite::Message>,
    #[allow(dead_code)]
    id: u32,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    init_logger();

    info!("Listening at ws://{LISTEN_ADDR}");
    let listener = TcpListener::bind(LISTEN_ADDR)
        .await
        .with_context(|| format!("Failed to bind at addr {LISTEN_ADDR}"))?;

    let clients = Arc::new(Mutex::new(HashMap::new()));

    let mut id_incr = 0;
    let mut next_id = || {
        id_incr += 1;
        id_incr
    };

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
        let client_id = next_id();
        clients.lock().await.insert(
            peer_addr.clone(),
            Client {
                send,
                id: client_id,
            },
        );
        tokio::spawn(async move {
            while let Ok(Some(msg)) = recv
                .try_next()
                .await
                .inspect_err(|_| info!("Client disconnected: {peer_addr}"))
            {
                // info!("received msg: {msg:#?}");
                let msg: ClientMessage =
                    serde_json::from_str(&msg.into_text().expect("Client sent a non text message"))
                        .expect("Client sent an invalid message");
                match msg {
                    ClientMessage::AcknowledgeChange(_uuid) => todo!(),
                    ClientMessage::Common(common_message) => {
                        match &common_message {
                            codlab::messages::CommonMessage::Change(change) => {
                                let change = &change.change.content_changes[0];
                                if let Some(range) = change.range {
                                    debug!(
                                        "#{}: ({}:{}):({}:{}) {:#?}",
                                        client_id,
                                        range.start.line,
                                        range.start.character,
                                        range.end.line,
                                        range.end.character,
                                        change.text
                                    );
                                } else {
                                    debug!("#{}: {:#?}", client_id, change.text);
                                }
                            }
                        }
                        let msg = ServerMessage::Common(common_message);
                        debug!("Broadcasting message...!");
                        let mut lock = clients.lock().await;
                        let futs: Vec<_> = lock
                            .iter_mut()
                            .filter(|(addr, _)| addr != &&peer_addr)
                            .map(|(_, client)| {
                                client.send.send(tungstenite::Message::Text(
                                    serde_json::to_string(&msg)
                                        .expect("To be able to construct a json")
                                        .into(),
                                ))
                            })
                            .collect();
                        let peers = futs.len();
                        join_all(futs).await;
                        debug!("Broadcasted message to {} peers successfully!", peers);
                    }
                }
            }
            clients.lock().await.remove(&peer_addr);
        });
    }
    Ok(())
}
