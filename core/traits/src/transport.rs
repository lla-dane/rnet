use anyhow::Result;
use async_trait::async_trait;
use rnet_multiaddr::Multiaddr;
use std::net::SocketAddr;

#[async_trait]
pub trait ITransport<T>: Sized {
    async fn listen(addr: &Multiaddr) -> Result<Self>;
    async fn accept(&self) -> Result<(T, SocketAddr)>;
    async fn dial(addr: &Multiaddr) -> Result<T>;
}
