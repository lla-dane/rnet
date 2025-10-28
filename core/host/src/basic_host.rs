use std::{collections::HashMap, sync::Arc};

use anyhow::{Ok, Result};
use rnet_core_multiaddr::{Multiaddr, Protocol};
use rnet_core_traits::transport::Transport;
use rnet_tcp::TcpTransport;
use tracing::{debug, info};

use crate::{
    keys::rsa::RsaKeyPair,
    multiselect::{multiselect::Multiselect, mutilselect_com::MultiselectComm},
};

#[derive(Debug)]
pub struct BasicHost {
    pub transport: TcpTransport,
    pub key_pair: RsaKeyPair,
    pub peer_info: PeerInfo,
    pub peer_store: HashMap<String, PeerInfo>,
    pub stream_handlers: HashMap<String, String>,
    pub multiselect: Multiselect,
    pub multiselect_comm: MultiselectComm,
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
            peer_store: HashMap::new(),
            stream_handlers: HashMap::new(),
            multiselect: Multiselect {},
            multiselect_comm: MultiselectComm {},
        })
    }

    pub async fn run(self: Arc<Self>) -> Result<()> {
        loop {
            let (mut stream, _addr) = self.transport.accept().await?;
            info!("New connection received");

            let host = self.clone();
            tokio::spawn(async move {
                // Now we have got the stream, next the handshake happens
                // Lets make a stream handler here
                // Then we will feed the assigned stream handler the stream

                host.multiselect.handshake(&mut stream).await.unwrap();
            });
        }
    }

    pub async fn dial(&self, addr: &Multiaddr) -> Result<()> {
        let mut stream = TcpTransport::dial(addr).await?;

        // Now this dial function will complete the handshake
        // and return the stream back.

        // let mut buf = [0u8; 32];
        // let n = stream.read(&mut buf).await.unwrap();
        // let received = String::from_utf8_lossy(&buf[..n]).to_string();

        // if received == "HELLO-FROM-SERVER" {
        //     stream.write(b"HELLO-FROM-CLIENT").await.unwrap();
        //     debug!("HANDSHAKE COMPLETE");
        // }

        self.multiselect_comm.negotiate(&mut stream).await.unwrap();

        Ok(())
    }
}
