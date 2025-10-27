use anyhow::{Ok, Result};
use rnet_core_host::BasicHost;
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

    let host = BasicHost::new("127.0.0.1:0").await.unwrap();
    let args: Vec<String> = env::args().collect();
    let mut mode = "server".to_string();
    let mut destination = "";
    if args.len() > 1 {
        mode = "client".to_string();
        destination = &args[1];
    }

    if mode == "server".to_string() {
        info!(
            "Run in new terminal: cargo run --bin host {}",
            host.listen_addr
        );
        host.run().await.unwrap();
    } else {
        host.dial(destination).await?;
    }

    Ok(())
}
