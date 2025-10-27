use anyhow::{Ok, Result};
use rnet_core_multiaddr::{Multiaddr, Protocol};
use rnet_core_traits::transport::{Connection, Transport};
use rnet_tcp::TcpTransport;
use tracing::{debug, info};

use crate::keys::rsa::RsaKeyPair;

#[derive(Debug)]
pub struct BasicHost {
    pub transport: TcpTransport,
    pub key_pair: RsaKeyPair,
    pub peer_info: PeerInfo,
    // Stream Handlers: hashmap
}

#[derive(Debug)]
pub struct PeerInfo {
    pub peer_id: String,
    pub listen_addr: Multiaddr,
}

impl BasicHost {
    pub async fn new(listen_addr: &mut Multiaddr) -> Result<Self> {
        let listener = TcpTransport::listen(&listen_addr).await.unwrap();
        let local_addr = listener.get_local_addr().unwrap();
        let parts: Vec<&str> = local_addr.split(':').collect();

        // ------ LISTEN-ADDR--------
        debug!("Generating RSA keypair");
        let keypair = RsaKeyPair::generate()?;
        let peer_id = keypair.peer_id();

        listen_addr
            .replace_value_for_protocol("tcp", parts[1])
            .unwrap();

        listen_addr.push_proto(Protocol::P2P(peer_id.clone()));

        // ----------------------------

        info!("Host listening on: {:?}", listen_addr.to_string());

        Ok(BasicHost {
            transport: listener,
            key_pair: keypair,
            peer_info: PeerInfo {
                peer_id,
                listen_addr: listen_addr.clone(),
            },
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

    pub async fn dial(&self, addr: &Multiaddr) -> Result<()> {
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
