use anyhow::Result;
use async_trait::async_trait;

#[derive(Debug, Clone)]
pub enum ProtocolId {
    Floodsub,
    Gossipsub,
}

#[async_trait]
pub trait IPubsub {
    fn topic_ids(&self) -> Vec<String>;
    // Provide an mpsc channel here
    async fn subscribe(&self, topic_id: String) -> Result<()>;
    async fn unsubscribe(&self, topic_id: String) -> Result<()>;
    async fn publish(&self, topic_id: Vec<String>, data: Vec<u8>) -> Result<()>;
}

#[async_trait]
pub trait IPubsubRouter {
    fn add_peer(&mut self, peer_id: String, protocol_id: String) -> Result<()>;
    fn remove_peer(&mut self, peer_id: String) -> Result<()>;
    async fn handle_rpc(&mut self, rpc: Vec<u8>, sender_peer_id: String) -> Result<()>;
    async fn publish(&mut self, msg_forwarder: String, pubsub_msg: Vec<u8>) -> Result<()>;
    async fn join(&mut self, topic: String) -> Result<()>;
    async fn leave(&mut self, topic: String) -> Result<()>;
}
