mod cli;

use anyhow::Result;
use floodsub::{pubsub::FloodSub, subscription::SubscriptionAPI};
use identity::multiaddr::Multiaddr;
use muxer::mplex::{conn::AsyncHandler, stream::MplexStream};
use node::inner::NodeInner;
use std::sync::Arc;
use tracing::{debug, info};
use tracing_subscriber::EnvFilter;

use crate::cli::cli_loop;

const FLOODSUB: &str = "rnet/floodsub/0.0.1";

// TODO: Put these receiver loops in FLoodsub module only, and
// when something is received, just emit an event and receive
// them via `global_rx` in the application side
async fn receiver_loop(mut sub_api: SubscriptionAPI) {
    let topic = sub_api.topic_id.clone();
    loop {
        match sub_api.recv().await {
            Some(payload) => {
                let msg = String::from_utf8_lossy(&payload).to_string();
                println!("[{}]: {}", topic, msg);
            }
            None => {
                debug!("[{}]: Receiver loop ended", topic);
                break;
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new("trace"))
        .without_time()
        .with_target(false)
        .compact()
        .init();

    let mut listen_addr = Multiaddr::new("ip4/127.0.0.1/tcp/0").unwrap();
    let (host_mpsc_tx, global_rx, local_peer_info) =
        NodeInner::new(&mut listen_addr, vec![]).await.unwrap();

    // TODO: DO all these things inside
    let (floodsub, _) = FloodSub::new(&local_peer_info).await.unwrap();

    let handler_fs = floodsub.clone();
    let handler: AsyncHandler = Arc::new(move |stream: MplexStream| {
        let fs = handler_fs.clone();
        Box::pin(async move { fs.stream_handler(stream).await })
    });
    host_mpsc_tx
        .set_stream_handler(FLOODSUB, handler)
        .await
        .unwrap();

    info!("Run in new terminal: \ncargo run --bin floodsub --release");
    cli_loop(host_mpsc_tx, floodsub).await.unwrap();

    Ok(())
}
