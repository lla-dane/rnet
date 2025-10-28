use std::{collections::HashMap, sync::Arc};

use anyhow::{Ok, Result};
use rnet_multiaddr::{Multiaddr, Protocol};
use rnet_peer::peer_info::PeerInfo;
use rnet_tcp::{TcpConn, TcpTransport};
use rnet_traits::transport::Transport;
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

impl BasicHost {
    pub async fn new(listen_addr: &mut Multiaddr) -> Result<Arc<Self>> {
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

        Ok(Arc::new(BasicHost {
            transport: listener,
            key_pair: keypair,
            peer_info: PeerInfo {
                peer_id,
                listen_addr: listen_addr.clone().to_string(),
            },
            peer_store: HashMap::new(),
            stream_handlers: HashMap::new(),
            multiselect: Multiselect {},
            multiselect_comm: MultiselectComm {},
        }))
    }

    pub async fn run(self: &mut Arc<Self>) -> Result<()> {
        loop {
            let (mut stream, _addr) = self.transport.accept().await?;
            info!("New connection received");

            let mut host = self.clone();
            tokio::spawn(async move {
                // Now we have got the stream, next the handshake happens
                // Lets make a stream handler here
                // Then we will feed the assigned stream handler the stream

                host.sync(&mut stream, false).await.unwrap();

                // Eventually the syncing will end, and stream will be handed
                // over to the stream handler from this async task
            });
        }
    }

    pub async fn dial(self: &mut Arc<Self>, addr: &Multiaddr) -> Result<()> {
        let mut stream = TcpTransport::dial(addr).await?;

        // Now this dial function will complete the handshake
        // and return the stream back.

        self.sync(&mut stream, true).await.unwrap();

        Ok(())
    }

    pub async fn sync(
        self: &mut Arc<Self>,
        stream: &mut TcpConn,
        is_initiator: bool,
    ) -> Result<()> {
        if !is_initiator {
            self.multiselect
                .handshake(stream, &self.peer_info)
                .await
                .unwrap();
        } else {
            self.multiselect_comm
                .handshake(stream, &self.peer_info)
                .await
                .unwrap();
        }

        Ok(())
    }
}
