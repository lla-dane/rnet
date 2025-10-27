use std::net::SocketAddr;
use anyhow::Result;
use async_trait::async_trait;

#[async_trait]
pub trait Connection {
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize>;
    async fn write(&mut self, buf: &[u8]) -> Result<usize>;
    async fn close(&mut self) -> Result<()>;
}

#[async_trait]
pub trait Transport: Sized {
    type Conn: Connection;
    async fn listen(addr: &str) -> Result<Self>;
    async fn accept(&self) -> Result<(Self::Conn, SocketAddr)>;
    async fn dial(addr: &str) -> Result<Self::Conn>;
}
