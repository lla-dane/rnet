use anyhow::Result;
use async_trait::async_trait;
use std::net::SocketAddr;

use crate::multiaddr::Multiaddr;

#[async_trait]
pub trait ITransport<T>: Sized {
    async fn listen(addr: &Multiaddr) -> Result<Self>;
    async fn accept(&self) -> Result<(T, SocketAddr)>;
    async fn dial(&self, addr: &Multiaddr) -> Result<T>;
}
