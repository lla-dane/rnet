use floodsub::pubsub::FloodSub;
use identity::{keys::rsa::RsaKeyPair, multiaddr::Multiaddr, peer::PeerInfo, traits::core::ISwarm};

use muxer::mplex::{
    conn::AsyncHandler,
    headers::{build_frame, MuxedStreamFlag},
};
use security::conn::SecureConn;
use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
};
use swarm::{inner::SwarmInner, swarm::Swarm};
use tokio::sync::{
    mpsc::{self, Receiver, Sender},
    Mutex,
};
use transport::{raw_conn::RawConnection, tcp::transport::TcpTransport};

use anyhow::Result;
use identity::peer::PeerData;
use identity::traits::{
    core::{IMultistream, IReadWriteClose},
    transport::ITransport,
};
use std::result::Result::Ok;
use tracing::{debug, info, warn};

use crate::{
    headers::{process_host_frame, HostMpscTxFlag},
    node::Node,
    protocol::InnerProtocolOpt,
};

const INTERNAL: [u8; 16] = *b"internal-payload";

// TODO: After sometime:
// Only one endpoint to connect with the whole rnet node: HostMpscTx
// Set protocols which are going to be active in host-initiation
// All the internal function calls will be event driven from HostMpscTx,
// and received from global_event_rx
// Integrate Swarm with BasicHost
// Fix the dependency tree

// Happy exams !!
pub struct NodeInner {
    pub key_pair: RsaKeyPair,
    pub peerstore: Arc<Mutex<PeerData>>,
    pub handlers: Arc<Mutex<HashMap<String, AsyncHandler>>>,

    pub node_mpsc_rx: Receiver<Vec<u8>>,
    pub swarm_mpsc_tx: Arc<Swarm>,
    pub global_event_tx: Sender<Vec<u8>>,
}

/// new
/// get_peer_id
/// get_addrs
/// get_pubkey
/// get_privkey
/// get_connected_peers
/// set_stream_handler
/// remove_stream_handler
/// disconnect
/// get_live_peers
/// is_peer_connected
impl NodeInner {
    pub async fn new(
        listen_addr: &mut Multiaddr,
        _protocol_opt: Vec<InnerProtocolOpt>,
    ) -> Result<(Arc<Node>, Receiver<Vec<u8>>, PeerInfo)> {
        // generate rsa-keypair
        // handlers
        // create mpsc channels
        // create/initiate swarm

        // keypair/handlers generate
        debug!("Generating RSA keypair");
        let keypair = RsaKeyPair::generate().unwrap();
        let peer_id = keypair.peer_id();
        let handlers = Arc::new(Mutex::new(HashMap::new()));

        // mpsc channels
        let (global_event_tx, global_event_rx) = mpsc::channel::<Vec<u8>>(100);
        let (mpsc_tx, node_mpsc_rx) = mpsc::channel::<Vec<u8>>(100);
        let node_mpsc_tx = Arc::new(Node {
            mpsc_tx,
            key_pair: keypair.clone(),
            handlers: handlers.clone(),
        });

        // swarm/local_peer_info
        let (swarm_mpsc_tx, peerstore, local_peer_info) = SwarmInner::new(
            "tcp",
            listen_addr,
            peer_id.clone(),
            handlers.clone(),
            global_event_tx.clone(),
        )
        .await
        .unwrap();

        info!("Node listening on: {}", listen_addr.to_string());

        let node_inner = NodeInner {
            key_pair: keypair,
            peerstore,
            handlers,
            node_mpsc_rx,
            swarm_mpsc_tx,
            global_event_tx,
        };

        tokio::spawn(async move {
            node_inner.run().await.unwrap();
        });

        Ok((node_mpsc_tx, global_event_rx, local_peer_info))
    }

    // migrate to swarm
    pub async fn run(mut self) -> Result<()> {
        let mut notification = VecDeque::<Vec<u8>>::new();

        loop {
            tokio::select! {
                Some(event) = self.node_mpsc_rx.recv() => {
                    notification.push_back(event);
                }

                _ = async {}, if !notification.is_empty() => {
                    let frame = notification.pop_front().unwrap();

                    // Process the data
                    let (flag, (str_pld, opt_vec_pld)): (HostMpscTxFlag, (String, Option<Vec<String>>)) = process_host_frame(frame).unwrap();
                    match flag {

                        HostMpscTxFlag::Connect => {
                            let maddr = Multiaddr::new(&str_pld).unwrap();
                            self.swarm_mpsc_tx.connect(&maddr).await.unwrap();
                        },

                        HostMpscTxFlag::NewStream => {
                            let maadr = Multiaddr::new(&str_pld).unwrap().to_string();
                            let protocols = opt_vec_pld.unwrap();
                            self.swarm_mpsc_tx.new_stream(&maadr, protocols).await.unwrap();
                        },

                        HostMpscTxFlag::Disconnect => {
                            let peer_id = &str_pld;
                            self.swarm_mpsc_tx.on_disconnect(peer_id).await.unwrap();
                        },
                    }
                }
            }
        }

        // TODO: ERROR HANDLING
    }

    async fn _execute_protocols(
        &self,
        protocol_opt: Vec<InnerProtocolOpt>,
        local_peer: &PeerInfo,
    ) -> Result<()> {
        for opt in protocol_opt {
            match opt {
                InnerProtocolOpt::Floodsub => {
                    let (floodsub, _) = FloodSub::new(local_peer).await.unwrap();
                    
                
                }
                InnerProtocolOpt::Ping => {}
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_headers() {
        let maddr = Multiaddr::new("ip4/127.0.0.1/tcp/0")
            .unwrap()
            .to_string()
            .into_bytes();

        let string = String::from_utf8(maddr).unwrap();
        println!("{}", string);
    }
}
