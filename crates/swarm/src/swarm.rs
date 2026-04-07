use async_trait::async_trait;
use identity::{keys::rsa::RsaKeyPair, multiaddr::Multiaddr, traits::core::ISwarm};

use tokio::sync::mpsc::Sender;

use anyhow::{Error, Result};
use std::result::Result::Ok;

use crate::headers::{build_host_frame, SwarmMpscTxFlag};

#[derive(Debug)]
pub struct Swarm {
    pub swarm_mpsc_tx: Sender<Vec<u8>>,
}

#[async_trait]
impl ISwarm for Swarm {
    async fn connect(&self, maddr: &Multiaddr) -> Result<()> {
        let frame = build_host_frame(SwarmMpscTxFlag::Connect, maddr.to_string(), None);
        self.write(frame).await.unwrap();
        Ok(())
    }

    async fn new_stream(&self, maddr: &str, protocols: Vec<String>) -> Result<()> {
        let frame = build_host_frame(
            SwarmMpscTxFlag::NewStream,
            maddr.to_string(),
            Some(protocols),
        );
        self.write(frame).await.unwrap();
        Ok(())
    }

    async fn on_disconnect(&self, peer_id: &str) -> Result<()> {
        let frame = build_host_frame(SwarmMpscTxFlag::Disconnect, peer_id.to_string(), None);
        self.write(frame).await.unwrap();
        Ok(())
    }

    async fn write(&self, notification: Vec<u8>) -> Result<()> {
        if let Err(e) = self.swarm_mpsc_tx.send(notification).await {
            return Err(Error::msg(format!("mpsc-receiver dropped: {}", e)));
        }

        Ok(())
    }
}
