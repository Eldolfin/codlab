use anyhow::Context;
use futures::{StreamExt, TryStreamExt as _};
use tokio::net::TcpListener;
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
    while let Ok((stream, peer_addr)) = listener.accept().await {
        info!("Client connected: {peer_addr}");
        let ws = match tokio_tungstenite::accept_async(stream).await {
            Ok(ws) => ws,
            Err(err) => {
                error!("Error in websocket handshake: {err:#}");
                break;
            }
        };
        let (send, mut recv) = ws.split();
        while let Ok(Some(msg)) = recv
            .try_next()
            .await
            .inspect_err(|err| error!("Failed to recv client messages: {err:#}"))
        {
            info!("received msg: {msg:#?}");
        }
    }
    Ok(())
}
