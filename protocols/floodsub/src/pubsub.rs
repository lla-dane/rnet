use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
};

use anyhow::Result;
use prost::Message as ProstMessage;
use rnet_mplex::mplex_stream::MuxedStream;
use rnet_peer::peer_info::PeerInfo;
use rnet_proto::floodsub::{rpc::SubOpts, Message, Rpc};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::{
    mpsc::{self, Receiver, Sender},
    Mutex,
};
use tracing::{debug, error, warn};

use crate::subscription::{process_floodsub_api_frame, SubAPIMpscFlag, SubscriptionAPI};

#[derive(Debug, Clone)]
pub struct FloosubStore {
    peer_topics: HashMap<String, Vec<String>>,
    peers: HashMap<String, Sender<Vec<u8>>>,
    subscribed_topic_api: HashMap<String, Sender<Vec<u8>>>,
}

#[derive(Debug, Clone)]
pub struct FloodSub {
    local_peer_info: PeerInfo,
    floodsub_mpsc_tx: Sender<Vec<u8>>,
    last_seen_cache: HashMap<Vec<u8>, u8>,
    floodsub_store: Arc<Mutex<FloosubStore>>,
}

impl FloodSub {
    pub async fn new(local_peer_info: PeerInfo) -> Result<(Self, Receiver<Vec<u8>>)> {
        let (floodsub_mpsc_tx, floodsub_mpsc_rx) = mpsc::channel::<Vec<u8>>(300);
        let floodsub = FloodSub {
            local_peer_info,
            floodsub_mpsc_tx,
            last_seen_cache: HashMap::new(),
            floodsub_store: Arc::new(Mutex::new(FloosubStore {
                peer_topics: HashMap::new(),
                peers: HashMap::new(),
                subscribed_topic_api: HashMap::new(),
            })),
        };

        Ok((floodsub, floodsub_mpsc_rx))
    }

    pub async fn handle_api(
        &self,
        mut floodsub_mpsc_rx: Receiver<Vec<u8>>,
        floodsub_store: Arc<Mutex<FloosubStore>>,
    ) {
        let mut notification = VecDeque::<Vec<u8>>::new();

        loop {
            tokio::select! {
                Some(event) = floodsub_mpsc_rx.recv() => {
                    notification.push_back(event);
                }

                _ = async {}, if !notification.is_empty() => {
                    let frame = notification.pop_front().unwrap();

                    let (flag, (_, opt_vec_pld, opt_data)): (SubAPIMpscFlag, (Option<String>, Option<Vec<String>>, Option<Vec<u8>>)) = process_floodsub_api_frame(frame).unwrap();
                    match flag {
                        SubAPIMpscFlag::DeadPeers => {
                            self.handle_dead_peers(opt_vec_pld.unwrap(),floodsub_store.clone())
                                .await.unwrap();
                        }
                        SubAPIMpscFlag::Unsubscribe => {
                            self._unsubscribe(opt_vec_pld.unwrap(), floodsub_store.clone()).await.unwrap();
                        }
                        SubAPIMpscFlag::Publish => {
                            let unix_timestamp: u64 = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
                            let ts_bytes = unix_timestamp.to_le_bytes().to_vec();

                            let pub_msg = Message {
                                from: Some(self.local_peer_info.clone().peer_id.as_bytes().to_vec()),
                                data: opt_data,
                                seqno: Some(ts_bytes),
                                topic_ids: opt_vec_pld.unwrap(),
                            };

                            self.publish(self.local_peer_info.clone().peer_id, pub_msg).await.unwrap();
                        }
                    }
                }
            }
        }
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

        Ok(())
    }

    pub async fn write_msg(&self, peer_id: String, rpc_msg: Rpc) -> Result<()> {
        let floodsub_peer_mpsc_tx = {
            let floodsub_store = self.floodsub_store.lock().await;
            floodsub_store.peers.get(&peer_id).unwrap().clone()
        };

        let mut buf = Vec::new();
        rpc_msg.encode(&mut buf).expect("Encoding failed");

        floodsub_peer_mpsc_tx.send(buf).await.unwrap();
        Ok(())
    }

    pub async fn subscribe(&mut self, topic_id: String) -> Option<SubscriptionAPI> {
        if self
            .topic_ids()
            .await
            .unwrap_or_else(|| vec![])
            .contains(&topic_id)
        {
            warn!("Already subscribed to the topic: {}", topic_id);
            return None;
        }

        debug!("Subscribing to topic: {}", topic_id);

        let (topic_mpsc_tx, topic_mpsc_rx) = mpsc::channel::<Vec<u8>>(100);
        let sub_api = SubscriptionAPI::new(
            topic_id.clone(),
            self.floodsub_mpsc_tx.clone(),
            topic_mpsc_rx,
        );

        {
            let mut store = self.floodsub_store.lock().await;
            store
                .subscribed_topic_api
                .insert(topic_id.clone(), topic_mpsc_tx);
        }

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

        let floodsub_store = self.floodsub_store.lock().await;
        for (_, stream) in floodsub_store.peers.iter() {
            stream.send(rpc_bytes.clone()).await.unwrap();
        }

        Ok(())
    }

    pub async fn unsubscribe(&mut self, topics: Vec<String>) -> Result<()> {
        self._unsubscribe(topics, self.floodsub_store.clone())
            .await
            .unwrap();
        Ok(())
    }

    pub async fn _unsubscribe(
        &self,
        topics: Vec<String>,
        floodsub_store: Arc<Mutex<FloosubStore>>,
    ) -> Result<()> {
        for topic_id in topics {
            if !self.topic_ids().await.unwrap_or(vec![]).contains(&topic_id) {
                warn!("Not subscribed to the topic: {}", topic_id);
                return Ok(());
            }

            debug!("Unsubsctibing from topic: {}", topic_id);

            let mut store = floodsub_store.lock().await;
            store.subscribed_topic_api.remove_entry(&topic_id);

            let mut unsub_rpc = Rpc::default();
            unsub_rpc.subscriptions.push(SubOpts {
                subscribe: Some(false),
                topic_id: Some(topic_id),
            });

            self.message_all_peers(unsub_rpc).await.unwrap();
        }
        Ok(())
    }

    pub async fn publish(&self, msg_forwarder_id: String, publish_msg: Message) -> Result<()> {
        let peers = self
            .get_peers_to_send(
                publish_msg.topic_ids.clone(),
                msg_forwarder_id,
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
        let mut floodsub_store = self.floodsub_store.lock().await;
        let topic_id = sub_msg.topic_id.clone().unwrap();

        if sub_msg.subscribe() {
            floodsub_store
                .peer_topics
                .entry(topic_id)
                .and_modify(|peers| {
                    if !peers.contains(&origin_id) {
                        peers.push(origin_id.clone());
                    }
                })
                .or_insert_with(|| vec![origin_id]);
        } else {
            if let Some(peers) = floodsub_store.peer_topics.get_mut(&topic_id) {
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
            let mut floodsub_store = self.floodsub_store.lock().await;
            floodsub_store
                .peers
                .insert(peer_id.clone(), floodsub_peer_mpsc_tx.clone());
        }

        // send in the hello-packet
        match self.get_hello_packet().await {
            None => {}
            Some(rpc) => {
                let mut buf = Vec::new();
                rpc.encode(&mut buf).expect("Encoding failed");

                floodsub_peer_mpsc_tx.send(buf).await.unwrap();
            }
        }

        debug!("New peer connected for Floodsub: {}", peer_id);

        Ok(())
    }

    pub async fn handle_dead_peers(
        &self,
        peer_ids: Vec<String>,
        floodsub_store: Arc<Mutex<FloosubStore>>,
    ) -> Result<()> {
        let mut store = floodsub_store.lock().await;

        for peer_id in peer_ids {
            if store.peers.remove(&peer_id).is_none() {
                return Ok(());
            }

            for peers in store.peer_topics.values_mut() {
                peers.retain(|peer| peer != &peer_id);
            }

            warn!("Dead peer in Floosub removed: {}", peer_id);
        }

        Ok(())
    }

    async fn get_peers_to_send(
        &self,
        topic_ids: Vec<String>,
        msg_forwarder: String,
        origin: String,
    ) -> Option<Vec<String>> {
        let floodsub_store = self.floodsub_store.lock().await;
        let known = [msg_forwarder, origin];
        let mut peers_to_send = Vec::new();

        for topic in topic_ids {
            match floodsub_store.peer_topics.get(&topic) {
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

    pub async fn topic_ids(&self) -> Option<Vec<String>> {
        let topics: Vec<String> = {
            let store = self.floodsub_store.lock().await;
            store
                .subscribed_topic_api
                .keys()
                .map(|x| x.clone())
                .collect()
        };

        if !topics.is_empty() {
            return Some(topics);
        }

        None
    }

    pub async fn get_hello_packet(&self) -> Option<Rpc> {
        let mut rpc = Rpc::default();
        match self.topic_ids().await {
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
