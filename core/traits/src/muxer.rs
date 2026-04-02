use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;

use crate::core::IHostMpscTx;

#[async_trait]
pub trait IMuxedConn: Send + Sync {
    async fn handle_incoming(&mut self, frames: Vec<u8>) -> Result<()>;
    async fn conn_handler(
        &mut self,
        peer_id: &str,
        host_mpsc_tx: Arc<dyn IHostMpscTx + Send + Sync>,
    ) -> Result<()>;
    async fn send(&self, msg: Vec<u8>) -> Result<()>;
    #[allow(clippy::ptr_arg)]
    async fn write(&mut self, msg: &Vec<u8>) -> Result<()>;
}

#[async_trait]
pub trait IMuxedStream {
    #[allow(clippy::ptr_arg)]
    async fn write(&self, msg: &Vec<u8>) -> Result<()>;
    async fn read(&mut self) -> Result<Vec<u8>>;

    // Merge these 2 handshake functions in the future
    async fn server_handshake(mut self) -> Result<()>;
    async fn client_handshake(mut self, protocol: Vec<u8>) -> Result<()>;
}
