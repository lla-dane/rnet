mod cli;

use anyhow::Result;
use rnet_floodsub::{pubsub::FloodSub, subscription::SubscriptionAPI};
use rnet_host::basic_host::BasicHost;
use rnet_mplex::{mplex::AsyncHandler, mplex_stream::MplexStream};
use rnet_multiaddr::Multiaddr;
use std::sync::Arc;
use tracing::{debug, info};
use tracing_subscriber::EnvFilter;

use crate::cli::cli_loop;

const FLOODSUB: &str = "/floodsub/1.0.0";

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
    let (mut host, host_tx) = BasicHost::new(&mut listen_addr).await.unwrap();

    let local_peer_info = {
        let peer_data = host.peer_data.lock().await;
        let peer_info = peer_data.peer_info.clone();
        peer_info
    };

    // TODO: DO all these things inside
    let (floodsub, _) = FloodSub::new(local_peer_info).await.unwrap();

    let handler_fs = floodsub.clone();
    let handler: AsyncHandler = Arc::new(move |stream: MplexStream| {
        let fs = handler_fs.clone();
        Box::pin(async move { fs.stream_handler(stream).await })
    });
    host.set_stream_handler(FLOODSUB, handler).unwrap();
    // ---------------

    let handle = tokio::spawn(async move {
        host.run().await.unwrap();
    });

    info!("Run in new terminal: \ncargo run --bin floodsub --release");
    cli_loop(host_tx, floodsub).await.unwrap();

    if let (Err(e),) = tokio::join!(handle) {
        return Err(e.into());
    }

    Ok(())
}
