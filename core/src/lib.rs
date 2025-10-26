use std::{fmt::Debug, net::SocketAddr};

use anyhow::Result;
use async_trait::async_trait;

/// Respresents a connection that can read/write bytes asynchronously
#[async_trait]
pub trait Connection: Send + Sync + Unpin {
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize>;
    async fn write(&mut self, data: &[u8]) -> Result<usize>;
    async fn close(&mut self) -> Result<()>;
}

#[async_trait]
pub trait Transport: Send + Sync + Debug {
    type Conn: Connection;

    /// Listen for incoming connections.
    async fn listen(addr: &str) -> Result<Self>
    where
        Self: Sized;

    /// Accept a single connection
    async fn accept(&mut self) -> Result<(Self::Conn, SocketAddr)>;

    /// Dial a remote peer and establish a connection.
    async fn dial(addr: &str) -> Result<Self::Conn>;
}
