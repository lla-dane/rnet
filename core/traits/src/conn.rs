// raw conn
// Secure conn
// muxed conn

use anyhow::Result;
use async_trait::async_trait;

#[async_trait]
pub trait IRawConnection<T> {
    async fn read(&mut self) -> Result<Vec<u8>>;
    async fn write(&mut self, msg: &Vec<u8>) -> Result<()>;
    async fn close(&mut self) -> Result<()>;
    fn peer_info(&self) -> T;
}

#[async_trait]
pub trait IMuxedConn {
    async fn handle_incoming(&mut self, frames: Vec<u8>) -> Result<()>;
    async fn write(&self, msg: Vec<u8>) -> Result<()>;
}
