use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
};

use anyhow::Result;
use prost::Message as ProstMessage;
use rnet_mplex::mplex_stream::MuxedStream;
use rnet_proto::floodsub::Rpc;
use tokio::sync::{
    mpsc::{self, Sender},
    Mutex,
};
use tracing::{debug, error, warn};

#[derive(Debug, Clone)]
pub struct FloosubPeers {
    peer_topics: HashMap<String, Vec<String>>,
    peers: HashMap<String, Sender<Vec<u8>>>,
}

#[derive(Debug, Clone)]
pub struct FloodSub {
    host_mpsc_tx: Sender<Vec<u8>>,
    last_seen_cache: HashMap<Vec<u8>, u8>,
    peerstore: Arc<Mutex<FloosubPeers>>,
}

impl FloodSub {
    pub fn new(host_mpsc_tx: Sender<Vec<u8>>) -> Result<Self> {
        Ok(FloodSub {
            host_mpsc_tx,
            last_seen_cache: HashMap::new(),
            peerstore: Arc::new(Mutex::new(FloosubPeers {
                peer_topics: HashMap::new(),
                peers: HashMap::new(),
            })),
        })
    }

    pub async fn handle_incoming(&self, rpc: Rpc, peer_id: String) -> Result<()> {
        if !rpc.publish.is_empty() {
            for msg in rpc.publish {
                // is msg not in subscribed topics: continue

                // else push this floodsub msg to others: self.router.publish
                // and send it in the mspc_tx of the subscription api
                debug!("received `publish` message {:?} from peer {}", msg, peer_id)
            }
        }

        if !rpc.subscriptions.is_empty() {
            for msg in rpc.subscriptions {
                debug!("Received subscription msg {:?} from peer {}", msg, peer_id)
                // self.handle_subscriptions(remote_peer_id, msg)
            }
        }
        Ok(())
    }

    pub async fn stream_handler(&mut self, stream: &mut MuxedStream) -> Result<()> {
        let peer_id = stream.remote_peer_info.clone().peer_id;
        let (floodsub_peer_mpsc_tx, mut floodsub_peer_mpsc_rx) = mpsc::channel::<Vec<u8>>(100);
        let mut notification = VecDeque::<Vec<u8>>::new();

        {
            let mut peerstore = self.peerstore.lock().await;
            peerstore
                .peers
                .insert(peer_id.clone(), floodsub_peer_mpsc_tx);
        }

        loop {
            tokio::select! {

                incoming = stream.read() => {
                    match incoming {
                        Ok(incoming) => {
                            let rpc = Rpc::decode(&incoming[..]).expect("Decoding failed");
                            self.handle_incoming(rpc, peer_id.clone()).await.unwrap();
                        }
                        Err(e) => {
                            warn!("Connection dropped: {}", e);
                            break;
                        }
                    }
                }

                Some(event) = floodsub_peer_mpsc_rx.recv() => {
                    notification.push_back(event);
                }

                _ = async {}, if !notification.is_empty() => {
                    let data = notification.pop_front().unwrap();
                    if let Err(e) = stream.write(&data).await {
                        error!("Error while writing in stream of peer: {}, {}", peer_id, e);
                        break;
                    }
                }
            }
        }

        Ok(())
    }

    pub async fn write_msg(&self, peer_id: String, rpc_msg: Rpc) -> Result<()> {
        let floodsub_peer_mpsc_tx = {
            let peerstore = self.peerstore.lock().await;
            peerstore.peers.get(&peer_id).unwrap().clone()
        };

        let mut buf = Vec::new();
        rpc_msg.encode(&mut buf).expect("Encoding failed");

        floodsub_peer_mpsc_tx.send(buf).await.unwrap();
        Ok(())
    }

    
}
