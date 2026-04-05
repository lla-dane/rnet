use async_trait::async_trait;
use identity::{keys::rsa::RsaKeyPair, multiaddr::Multiaddr};

use tokio::sync::mpsc::Sender;

use anyhow::{Error, Result};
use identity::traits::core::INode;
use std::result::Result::Ok;

use crate::headers::{build_host_frame, HostMpscTxFlag};

#[derive(Debug)]
pub struct Node {
    pub key_pair: RsaKeyPair,
    pub host_mpsc_tx: Sender<Vec<u8>>,
}

#[async_trait]
impl INode for Node {
    async fn connect(&self, maddr: &Multiaddr) -> Result<()> {
        let frame = build_host_frame(HostMpscTxFlag::Connect, maddr.to_string(), None);
        self.write(frame).await.unwrap();
        Ok(())
    }

    async fn new_stream(&self, maddr: &str, protocols: Vec<String>) -> Result<()> {
        let frame = build_host_frame(
            HostMpscTxFlag::NewStream,
            maddr.to_string(),
            Some(protocols),
        );
        self.write(frame).await.unwrap();
        Ok(())
    }

    async fn on_disconnect(&self, peer_id: &str) -> Result<()> {
        let frame = build_host_frame(HostMpscTxFlag::Disconnect, peer_id.to_string(), None);
        self.write(frame).await.unwrap();
        Ok(())
    }

    async fn write(&self, notification: Vec<u8>) -> Result<()> {
        if let Err(e) = self.host_mpsc_tx.send(notification).await {
            return Err(Error::msg(format!("mpsc-receiver dropped: {}", e)));
        }

        Ok(())
    }
}
