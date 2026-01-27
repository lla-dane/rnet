use anyhow::Result;
use async_trait::async_trait;
use rnet_multiaddr::Multiaddr;
use std::net::SocketAddr;

#[async_trait]
pub trait IReadWriteClose {
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize>;
    async fn read_exact(&mut self, buf: &mut [u8]) -> Result<()>;
    async fn recv_msg(&mut self) -> Result<Vec<u8>>;
    async fn send_bytes(&mut self, msg: &Vec<u8>) -> Result<()>;
    async fn write(&mut self, buf: &[u8]) -> Result<usize>;
    async fn close(&mut self) -> Result<()>;
}

#[async_trait]
pub trait ITransport<T>: Sized
where
    T: IReadWriteClose,
{
    async fn listen(addr: &Multiaddr) -> Result<Self>;
    async fn accept(&self) -> Result<(T, SocketAddr)>;
    async fn dial(addr: &Multiaddr) -> Result<T>;
}
