mod cli;

use anyhow::Result;
use identity::multiaddr::Multiaddr;
use node::{inner::NodeInner, protocol::InnerProtocolOpt};
use tracing::info;
use tracing_subscriber::EnvFilter;

use crate::cli::cli_loop;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new("trace"))
        .without_time()
        .with_target(false)
        .compact()
        .init();

    let mut listen_addr = Multiaddr::new("ip4/127.0.0.1/tcp/0").unwrap();
    let (host_mpsc_tx, _global_rx) = NodeInner::new(
        &mut listen_addr,
        vec![InnerProtocolOpt::Floodsub, InnerProtocolOpt::Ping],
    )
    .await
    .unwrap();

    info!("Run in new terminal: \ncargo run --bin floodsub --release");
    cli_loop(host_mpsc_tx).await.unwrap();

    Ok(())
}
