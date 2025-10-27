use std::env;
use std::time::Duration;

use anyhow::{Ok, Result};
use rnet_core_traits::transport::{Connection, Transport};
use rnet_tcp::TcpTransport;
use tokio::time::sleep;
use tracing::{debug, info};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new("trace"))
        .without_time()
        .with_target(false) 
        .compact()
        .init();
    let args: Vec<String> = env::args().collect();
    let mut mode = "server".to_string();
    let mut destination = "";
    if args.len() > 1 {
        mode = "client".to_string();
        destination = &args[1];
    }

    if mode == "server".to_string() {
        let listener = TcpTransport::listen("127.0.0.1:0").await.unwrap();
        debug!(
            "Run in another terminal: cargo run -- {}",
            listener.get_local_addr().unwrap()
        );
        loop {
            let (mut stream, _addr) = listener.accept().await.unwrap();

            tokio::spawn(async move {
                let mut buf = [0u8; 16];
                let n = stream.read(&mut buf).await.unwrap();

                let received = String::from_utf8_lossy(&buf[..n]).to_string();
                info!("Recieved {:?}", received);

                debug!("Processing");
                sleep(Duration::from_secs(3)).await;

                stream.write(b"pong").await.unwrap();
                info!("Responded with pong");
            });
        }
    } else {
        let mut stream = TcpTransport::dial(destination).await.unwrap();
        info!("Sending ping");
        stream.write(b"ping").await.unwrap();

        let mut buf = [0u8; 16];
        let n = stream.read(&mut buf).await.unwrap();

        let received = String::from_utf8_lossy(&buf[..n]).to_string();
        info!("Received {:?}", received);
    }

    Ok(())
}
