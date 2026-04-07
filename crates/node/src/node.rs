use async_trait::async_trait;
use identity::{keys::rsa::RsaKeyPair, multiaddr::Multiaddr};

use muxer::mplex::conn::AsyncHandler;
use tokio::sync::{mpsc::Sender, Mutex};

use anyhow::{Error, Result};
use identity::traits::core::INode;
use std::{collections::HashMap, result::Result::Ok, sync::Arc};

use crate::headers::{build_host_frame, HostMpscTxFlag};

pub struct Node {
    pub key_pair: RsaKeyPair,
    pub mpsc_tx: Sender<Vec<u8>>,
    pub handlers: Arc<Mutex<HashMap<String, AsyncHandler>>>,
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
        if let Err(e) = self.mpsc_tx.send(notification).await {
            return Err(Error::msg(format!("mpsc-receiver dropped: {}", e)));
        }

        Ok(())
    }
}

impl Node {
    pub async fn set_stream_handler(&self, protocol: &str, handler: AsyncHandler) -> Result<()> {
        let mut handler_registry = self.handlers.lock().await;
        handler_registry.insert(protocol.to_string(), handler);

        Ok(())
    }
}
