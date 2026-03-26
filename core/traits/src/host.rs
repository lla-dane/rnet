use anyhow::Result;
use async_trait::async_trait;
use rnet_multiaddr::Multiaddr;

#[async_trait]
pub trait IHostMpscTx {
    async fn connect(&self, maddr: &Multiaddr) -> Result<()>;
    async fn new_stream(&self, maddr: &str, protocols: Vec<String>) -> Result<()>;
    async fn on_disconnect(&self, peer_id: &str) -> Result<()>;
    async fn write(&self, notification: Vec<u8>) -> Result<()>;
}
