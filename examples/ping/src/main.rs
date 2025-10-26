use std::time::Duration;

use anyhow::Result;
use rnet_core::Connection;
use rnet_core::Transport;
use rnet_transport_tcp::TcpTransport;

#[tokio::main]
async fn main() -> Result<()> {
    // Start the server in separate task =
    tokio::spawn(async {
        let mut server = TcpTransport::listen("127.0.0.1:7000").await.unwrap();
        println!("Server listening on 127.0.0.1:7000");

        let (mut conn, addr) = server.accept().await.unwrap();
        println!("Accepted connection from {}", addr);

        let mut buf = [0u8; 16];
        let n = conn.read(&mut buf).await.unwrap();
        println!("Server received: {}", String::from_utf8_lossy(&buf[..n]));

        conn.write(b"pong").await.unwrap();
    });

    // Give server time to start
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Client connects
    let mut conn = TcpTransport::dial("127.0.0.1:7000").await?;
    conn.write(b"ping").await?;

    let mut buf = [0u8; 16];
    let n = conn.read(&mut buf).await?;
    println!("Client received: {}", String::from_utf8_lossy(&buf[..n]));

    Ok(())
}
