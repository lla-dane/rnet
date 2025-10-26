use anyhow::{Ok, Result};
use rnet_core::Connection;
use rnet_core::Transport;
use rnet_tcp::TcpTransport;

#[tokio::main]
async fn main() -> Result<()> {
    tokio::spawn(async {
        let listener = TcpTransport::listen("127.0.0.1:8080").await.unwrap();
        let (mut stream, _addr) = listener.accept().await.unwrap();

        let mut buf = [0; 16];
        let n = stream.read(&mut buf).await.unwrap();

        let received = String::from_utf8_lossy(&buf[..n]).to_string();
        if received == "ping" {
            println!("Server received ping");
            stream.write(b"pong").await.unwrap();
        }
    });

    let mut stream = TcpTransport::dial("127.0.0.1:8080").await.unwrap();
    stream.write(b"ping").await.unwrap();

    let mut buf = [0u8; 16];
    let n = stream.read(&mut buf).await.unwrap();

    let received = String::from_utf8_lossy(&buf[..n]).to_string();
    if received == "pong" {
        println!("Client received pong");
    }

    Ok(())
}
