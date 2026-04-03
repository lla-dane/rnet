use anyhow::Result;
use async_trait::async_trait;
use rnet_multiaddr::Multiaddr;

/// Transport stream have to implement this trait
/// i.e `TcpStream` `UdpStream` `QuicStream`
#[async_trait]
pub trait IReadWriteClose {
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize>;
    async fn read_exact(&mut self, buf: &mut [u8]) -> Result<()>;
    async fn recv_msg(&mut self) -> Result<Vec<u8>>;

    #[allow(clippy::ptr_arg)]
    async fn send_bytes(&mut self, msg: &Vec<u8>) -> Result<()>;
    async fn write(&mut self, buf: &[u8]) -> Result<usize>;
    async fn close(&mut self) -> Result<()>;
}

#[async_trait]
pub trait IRawConnection {
    async fn read(&mut self) -> Result<Vec<u8>>;

    #[allow(clippy::ptr_arg)]
    async fn write(&mut self, msg: &Vec<u8>) -> Result<()>;
    async fn close(&mut self) -> Result<()>;
}

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
    async fn identify(&self, local_peer_info: &X, stream: &mut T, is_intitiator: bool)
        -> Result<X>;
}

pub trait IKeys<T> {
    fn public_key(&self) -> String;
    fn sign(&self, msg: &[u8]) -> Result<T>;
    fn verify(&self, msg: &[u8], sig: &T) -> bool;
}
