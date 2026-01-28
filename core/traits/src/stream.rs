// Tcp Stream
// udp stream
// quic stream
// mplex stream
// yamux stream
use anyhow::Result;
use async_trait::async_trait;

/// Transport stream have to implement this trait
/// i.e `TcpStream` `UdpStream` `QuicStream` 
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
pub trait IMuxedStream {
    async fn write(&self, msg: &Vec<u8>) -> Result<()>;
    async fn read(&mut self) -> Result<Vec<u8>>;

    // Merge these 2 handshake functions in the future
    async fn server_handshake(mut self) -> Result<()>;
    async fn client_handshake(mut self, protocol: Vec<u8>) -> Result<()>; 
}
