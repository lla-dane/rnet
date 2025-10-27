use std::env;
use std::time::Duration;

use anyhow::{Ok, Result};
use rnet_core::Connection;
use rnet_core::Transport;
use rnet_tcp::TcpTransport;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    let mut mode = "server".to_string();
    let mut destination = "";
    if args.len() > 1 {
        mode = "client".to_string();
        destination = &args[1];
    }

    if mode == "server".to_string() {
        let listener = TcpTransport::listen("127.0.0.1:8080").await.unwrap();
        println!("Run: \ncargo run -- 127.0.0.1:8080");
        loop {
            let (mut stream, _addr) = listener.accept().await.unwrap();

            tokio::spawn(async move {
                let mut buf = [0u8; 16];
                let n = stream.read(&mut buf).await.unwrap();

                let received = String::from_utf8_lossy(&buf[..n]).to_string();
                println!("Recieved {:?}", received);

                println!("Processing");
                sleep(Duration::from_secs(3)).await;

                stream.write(b"pong").await.unwrap();
                println!("Responded with pong");
            });
        }
    } else {
        let mut stream = TcpTransport::dial(destination).await.unwrap();
        println!("Sending ping");
        stream.write(b"ping").await.unwrap();

        let mut buf = [0u8; 16];
        let n = stream.read(&mut buf).await.unwrap();

        let received = String::from_utf8_lossy(&buf[..n]).to_string();
        println!("Received {:?}", received);
    }
    Ok(())
}
