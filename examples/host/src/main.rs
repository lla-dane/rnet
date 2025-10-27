use anyhow::{Ok, Result};
use rnet_core_host::basic_host::BasicHost;
use rnet_core_multiaddr::Multiaddr;
use std::env;
use tracing::info;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new("trace"))
        .without_time()
        .with_target(false)
        .compact()
        .init();

    let mut listen_addr = Multiaddr::new("ip4/127.0.0.1/tcp/0").unwrap();
    let host = BasicHost::new(&mut listen_addr).await.unwrap();
    let args: Vec<String> = env::args().collect();
    let mut mode = "server".to_string();
    let mut destination = "";
    if args.len() > 1 {
        mode = "client".to_string();
        destination = &args[1];
    }

    if mode == "server".to_string() {
        info!(
            "Run in new terminal: \ncargo run --bin host {:?}",
            host.peer_info.listen_addr.to_string()
        );
        host.run().await.unwrap();
    } else {
        let multiaddr = Multiaddr::new(destination).unwrap();
        host.dial(&multiaddr).await?;
    }

    Ok(())
}
