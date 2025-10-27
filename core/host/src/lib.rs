pub mod keys;

use anyhow::{Ok, Result};
use rnet_core_traits::transport::{Connection, Transport};
use rnet_tcp::TcpTransport;
use tracing::{debug, info};

pub struct BasicHost {
    pub transport: TcpTransport,
    pub listen_addr: String,
    // Stream Handlers: hashmap
    // peer_id
}

impl BasicHost {
    pub async fn new(listen_addr: &str) -> Result<Self> {
        let listener = TcpTransport::listen(&listen_addr).await.unwrap();
        let listen_addr = listener.get_local_addr().unwrap();
        info!("Listener listening on: {:?}", listen_addr);

        Ok(BasicHost {
            transport: listener,
            listen_addr: listen_addr,
        })
    }

    pub async fn run(&self) -> Result<()> {
        loop {
            let (mut stream, _addr) = self.transport.accept().await?;
            debug!("NEW CONNECTION RECEIVED");
            tokio::spawn(async move {
                // Now we have got the stream, next the handshake happens
                // Lets make a stream handler here
                // Then we will feed the assigned stream handler the stream

                stream.write(b"HELLO-FROM-SERVER").await.unwrap();
                let mut buf = [0u8; 32];
                let n = stream.read(&mut buf).await.unwrap();
                let received = String::from_utf8_lossy(&buf[..n]).to_string();

                if received == "HELLO-FROM-CLIENT" {
                    debug!("HANDSHAKE COMPLETE");
                }
            });
        }
    }

    pub async fn dial(&self, addr: &str) -> Result<()> {
        let mut stream = TcpTransport::dial(addr).await?;

        // Now this dial function will complete the handshake
        // and return the stream back.

        let mut buf = [0u8; 32];
        let n = stream.read(&mut buf).await.unwrap();
        let received = String::from_utf8_lossy(&buf[..n]).to_string();

        if received == "HELLO-FROM-SERVER" {
            stream.write(b"HELLO-FROM-CLIENT").await.unwrap();
            debug!("HANDSHAKE COMPLETE");
        }

        Ok(())
    }
}
