use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
};

use anyhow::Result;
use prost::Message as ProstMessage;
use rnet_mplex::mplex_stream::MuxedStream;
use rnet_proto::floodsub::{rpc::SubOpts, Message, Rpc};
use tokio::sync::{
    mpsc::{self, Sender},
    Mutex,
};
use tracing::{debug, error, warn};

use crate::subscription::SubscriptionAPI;

#[derive(Debug, Clone)]
pub struct FloosubPeers {
    peer_topics: HashMap<String, Vec<String>>,
    peers: HashMap<String, Sender<Vec<u8>>>,
}

#[derive(Debug, Clone)]
pub struct FloodSub {
    floodsub_mpsc_tx: Sender<Vec<u8>>,
    last_seen_cache: HashMap<Vec<u8>, u8>,
    peerstore: Arc<Mutex<FloosubPeers>>,
    subscribed_topic_api: HashMap<String, Sender<Vec<u8>>>,
}

impl FloodSub {
    pub fn new() -> Result<Self> {
        let (floodsub_mpsc_tx, floodsub_mpsc_rx) = mpsc::channel::<Vec<u8>>(300);

        Ok(FloodSub {
            floodsub_mpsc_tx,
            last_seen_cache: HashMap::new(),
            peerstore: Arc::new(Mutex::new(FloosubPeers {
                peer_topics: HashMap::new(),
                peers: HashMap::new(),
            })),
            subscribed_topic_api: HashMap::new(),
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

        // We received a new peer here, so send a hello packet too,
        // to notify them of our subscribed topics
        self.handle_new_peer(floodsub_peer_mpsc_tx, peer_id.clone())
            .await
            .unwrap();

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

        // TODO: handle dead peer
        // - remove from peers.floodsub
        // - remove from peer-topics
        self.handle_dead_peers(vec![&peer_id]).await.unwrap();
        warn!("Dead peer in Floosub removed: {}", peer_id);

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

    pub async fn subscribe(&mut self, topic_id: String) -> Option<SubscriptionAPI> {
        debug!("Subscribing to topic: {}", topic_id);

        if self
            .topic_ids()
            .unwrap_or_else(|| vec![])
            .contains(&topic_id)
        {
            warn!("Already subscribed to the topic: {}", topic_id);
            return None;
        }

        let (topic_mpsc_tx, topic_mpsc_rx) = mpsc::channel::<Vec<u8>>(100);
        let sub_api = SubscriptionAPI::new(
            self.floodsub_mpsc_tx.clone(),
            topic_mpsc_tx.clone(),
            topic_mpsc_rx,
        );

        self.subscribed_topic_api
            .insert(topic_id.clone(), topic_mpsc_tx);

        let mut sub_rpc = Rpc::default();
        sub_rpc.subscriptions.push(SubOpts {
            subscribe: Some(true),
            topic_id: Some(topic_id),
        });

        self.message_all_peers(sub_rpc).await.unwrap();

        Some(sub_api)
    }

    pub async fn message_all_peers(&self, rpc: Rpc) -> Result<()> {
        let mut rpc_bytes = Vec::new();
        rpc.encode(&mut rpc_bytes).expect("Encoding failed");

        let peerstore = self.peerstore.lock().await;
        for (_, stream) in peerstore.peers.iter() {
            stream.send(rpc_bytes.clone()).await.unwrap();
        }

        Ok(())
    }

    pub async fn unsubscribe(&mut self) -> Result<()> {
        Ok(())
    }

    pub async fn publish(&self, msg_forwarder: String, publish_msg: Message) -> Result<()> {
        let peers = self
            .get_peers_to_send(
                publish_msg.topic_ids.clone(),
                msg_forwarder,
                String::from_utf8_lossy(publish_msg.from.as_ref().unwrap()).to_string(),
            )
            .await
            .unwrap_or_else(|| vec![]);

        let mut rpc_msg = Rpc::default();
        rpc_msg.publish.push(publish_msg.clone());

        debug!("Publishing message: {:?}", publish_msg);
        for peer_id in peers {
            self.write_msg(peer_id, rpc_msg.clone()).await.unwrap();
        }

        Ok(())
    }

    pub async fn handle_subopts(&mut self, origin_id: String, sub_msg: SubOpts) -> Result<()> {
        let mut peerstore = self.peerstore.lock().await;
        let topic_id = sub_msg.topic_id.clone().unwrap();

        if sub_msg.subscribe() {
            peerstore
                .peer_topics
                .entry(topic_id)
                .and_modify(|peers| {
                    if !peers.contains(&origin_id) {
                        peers.push(origin_id.clone());
                    }
                })
                .or_insert_with(|| vec![origin_id]);
        } else {
            if let Some(peers) = peerstore.peer_topics.get_mut(&topic_id) {
                peers.retain(|x| x != &origin_id);
            }
        }

        Ok(())
    }

    pub async fn handle_new_peer(
        &mut self,
        floodsub_peer_mpsc_tx: Sender<Vec<u8>>,
        peer_id: String,
    ) -> Result<()> {
        {
            let mut peerstore = self.peerstore.lock().await;
            peerstore
                .peers
                .insert(peer_id, floodsub_peer_mpsc_tx.clone());
        }

        // send in the hello-packet
        match self.get_hello_packet() {
            None => {}
            Some(rpc) => {
                let mut buf = Vec::new();
                rpc.encode(&mut buf).expect("Encoding failed");

                floodsub_peer_mpsc_tx.send(buf).await.unwrap();
            }
        }

        Ok(())
    }

    pub async fn handle_dead_peers(&mut self, peer_ids: Vec<&String>) -> Result<()> {
        let mut peerstore = self.peerstore.lock().await;

        for peer_id in peer_ids {
            if peerstore.peers.remove(peer_id).is_none() {
                return Ok(());
            }

            for peers in peerstore.peer_topics.values_mut() {
                peers.retain(|peer| peer != peer_id);
            }
        }

        Ok(())
    }

    async fn get_peers_to_send(
        &self,
        topic_ids: Vec<String>,
        msg_forwarder: String,
        origin: String,
    ) -> Option<Vec<String>> {
        let peerstore = self.peerstore.lock().await;
        let known = [msg_forwarder, origin];
        let mut peers_to_send = Vec::new();

        for topic in topic_ids {
            match peerstore.peer_topics.get(&topic) {
                Some(peers) => {
                    for peer in peers {
                        if !known.contains(peer) {
                            peers_to_send.push(peer.clone());
                        }
                    }
                }
                None => continue,
            }
        }

        if !peers_to_send.is_empty() {
            return Some(peers_to_send);
        }

        return None;
    }

    pub fn topic_ids(&self) -> Option<Vec<String>> {
        let topics: Vec<String> = self
            .subscribed_topic_api
            .keys()
            .map(|x| x.clone())
            .collect();

        if !topics.is_empty() {
            return Some(topics);
        }

        None
    }

    pub fn get_hello_packet(&self) -> Option<Rpc> {
        let mut rpc = Rpc::default();
        match self.topic_ids() {
            Some(topics) => {
                for topic_id in topics {
                    rpc.subscriptions.push(SubOpts {
                        subscribe: Some(true),
                        topic_id: Some(topic_id),
                    });
                }
                return Some(rpc);
            }
            None => {
                return None;
            }
        }
    }
}
