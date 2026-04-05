use anyhow::Result;
use identity::multiaddr::Multiaddr;
use identity::traits::core::INode;
use identity::traits::muxer::IMuxedStream;
use muxer::mplex::{conn::AsyncHandler, stream::MplexStream};
use node::inner::NodeInner;
use std::{env, sync::Arc};
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

const PING_LENGTH: usize = 32;
const IPFS_PING: &str = "/ipfs/ping/1.0.0";

pub async fn handle_ping(stream: &mut MplexStream) -> Result<()> {
    let payload = vec![0x01; PING_LENGTH];

    if stream.is_initiator {
        println!("sending ping to {}", stream.remote_peer_info.peer_id);
        stream.write(&payload).await.unwrap();

        let response = match stream.read().await {
            Ok(res) => res,
            Err(e) => {
                warn!("Connection dropped: {}", e);
                return Ok(());
            }
        };

        if response == payload {
            println!("received pong from {}", stream.remote_peer_info.peer_id);
        }
    } else {
        let req = match stream.read().await {
            Ok(req) => req,
            Err(e) => {
                warn!("Connection dropped: {}", e);
                return Ok(());
            }
        };

        if req == payload {
            println!("received ping from {}", stream.remote_peer_info.peer_id);
            println!("sending pong to {}", stream.remote_peer_info.peer_id);

            stream.write(&payload).await.unwrap();
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new("trace"))
        .without_time()
        .with_target(false)
        .compact()
        .init();
    let args: Vec<String> = env::args().collect();

    let mut listen_addr = Multiaddr::new("ip4/127.0.0.1/tcp/0").unwrap();
    let (mut host, host_tx, _global_rx) = NodeInner::new(&mut listen_addr, vec![]).await.unwrap();

    let mut mode = "server".to_string();
    let mut destination = "";
    if args.len() > 1 {
        mode = "client".to_string();
        destination = &args[1];
    }

    let peer_data = host.peerstore.lock().await.clone();

    let handler: AsyncHandler =
        Arc::new(|mut stream: MplexStream| Box::pin(async move { handle_ping(&mut stream).await }));
    host.set_stream_handler(IPFS_PING, handler).unwrap();

    let handle = tokio::spawn(async move {
        host.run().await.unwrap();
    });

    if mode == "server" {
        info!(
            "Run in new terminal: \ncargo run --bin ping --release {:?}",
            peer_data.peer_info.listen_addr.to_string()
        );
    } else {
        let multiaddr = Multiaddr::new(destination).unwrap();
        // host_tx.connect(&multiaddr).await?;

        host_tx
            .new_stream(&multiaddr.to_string(), vec![IPFS_PING.to_string()])
            .await
            .unwrap();
    }

    if let (Err(e),) = tokio::join!(handle) {
        return Err(e.into());
    }

    Ok(())
}
