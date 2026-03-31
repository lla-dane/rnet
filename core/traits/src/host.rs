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

#[async_trait]
pub trait IMultistream<T, W, X> {
    async fn handshake(&self, local_peer_info: &X, stream: T, is_intitiator: bool) -> Result<W>;
    async fn try_select(&self, stream: &mut T, proto: &str, is_intitiator: bool) -> Result<()>;
}

pub trait IKeys<T> {
    fn public_key(&self) -> String;
    fn sign(&self, msg: &[u8]) -> Result<T>;
    fn verify(&self, msg: &[u8], sig: &T) -> bool;
}
