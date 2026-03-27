// raw conn
// Secure conn
// muxed conn

use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;

use crate::host::IHostMpscTx;

#[async_trait]
pub trait ISecuredConn {
    async fn read(&mut self) -> Result<Vec<u8>>;

    #[allow(clippy::ptr_arg)]
    async fn write(&mut self, msg: &Vec<u8>) -> Result<()>;
    async fn close(&mut self) -> Result<()>;
}

#[async_trait]
pub trait IRawConnection<T> {
    async fn read(&mut self) -> Result<Vec<u8>>;

    #[allow(clippy::ptr_arg)]
    async fn write(&mut self, msg: &Vec<u8>) -> Result<()>;
    async fn close(&mut self) -> Result<()>;
    fn peer_info(&self) -> T;
}

#[async_trait]
pub trait IMuxedConn {
    async fn handle_incoming(&mut self, frames: Vec<u8>) -> Result<()>;
    async fn conn_handler<W>(mut self, peer_id: &str, host_mpsc_tx: &Arc<W>) -> Result<()>
    where
        W: IHostMpscTx + Send + Sync;
    async fn write(&self, msg: Vec<u8>) -> Result<()>;
}
