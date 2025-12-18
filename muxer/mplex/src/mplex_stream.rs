use anyhow::{Error, Ok, Result};
use rnet_peer::peer_info::PeerInfo;
use tokio::sync::mpsc::{Receiver, Sender};
use tracing::{debug, info, warn};

use crate::{headers::MuxedStreamFlag, mplex::build_frame};

#[derive(Debug)]
pub struct MuxedStream {
    conn_tx: Sender<Vec<u8>>,
    conn_rx: Receiver<Vec<u8>>,
    pub stream_id: u32,
    pub is_initiator: bool,
    inbox_tx: tokio::sync::mpsc::Sender<Vec<u8>>,
    inbox_rx: tokio::sync::mpsc::Receiver<Vec<u8>>,
    pub remote_peer_info: PeerInfo,
    pub protocols: Vec<String>,
}

impl MuxedStream {
    pub fn new(
        conn_tx: Sender<Vec<u8>>,
        conn_rx: Receiver<Vec<u8>>,
        stream_id: u32,
        is_initiator: bool,
        remote_peer_info: PeerInfo,
        protocols: Vec<String>,
    ) -> MuxedStream {
        let (tx, rx) = tokio::sync::mpsc::channel(100);
        MuxedStream {
            conn_tx,
            conn_rx,
            stream_id,
            is_initiator,
            inbox_rx: rx,
            inbox_tx: tx,
            remote_peer_info,
            protocols,
        }
    }

    pub async fn handle_incoming(&self, payload: Vec<u8>) -> Result<()> {
        self.inbox_tx.send(payload.clone()).await?;
        info!(
            "stream mpsc channel msg received here, {}",
            String::from_utf8(payload).unwrap()
        );
        Ok(())
    }

    pub async fn read(&mut self) -> Result<Vec<u8>> {
        self.inbox_rx
            .recv()
            .await
            .ok_or_else(|| anyhow::anyhow!("Stream closed"))
    }

    pub async fn write(&self, msg: &Vec<u8>) -> Result<()> {
        {
            let frame = build_frame(self.stream_id, MuxedStreamFlag::MessageRequest, msg);
            self.conn_tx.send(frame).await?
        }

        Ok(())
    }

    pub async fn server_handshake(&mut self) -> Result<()> {
        let payload = self.conn_rx.recv().await.unwrap();

        if payload != b"protocol1".to_vec() {
            return Err(Error::msg("Server: Protocol negotiation failed"));
        }

        let frame = build_frame(
            self.stream_id,
            MuxedStreamFlag::HandshakeRes,
            &b"protocol1".to_vec(),
        );
        self.conn_tx.send(frame).await?;

        debug!("[SERVER]: PROTOCOL HANDSHAKE FINISHED");

        Ok(())
    }

    pub async fn client_handshake(&mut self) -> Result<()> {
        // let protocols = self.protocols.clone();

        for _ in vec!["protocol1"] {
            let frame = build_frame(
                self.stream_id,
                MuxedStreamFlag::HandshakeReq,
                &b"protocol1".to_vec(),
            );

            self.conn_tx.send(frame).await?;
            warn!("[CLIENT]: PAYLOAD WENT IN");

            let res = self.conn_rx.recv().await.unwrap();

            if b"protocol1".to_vec() != res {
                return Err(Error::msg("Client: Protocol negotiation failed"));
            } else {
                info!("[CLIENT]: CLIENT: FINISHED");
                break;
            }
        }
        debug!("[CLIENT]: PROTOCOL HANDSHAKE FINISHED");

        Ok(())
    }

    pub fn peer_info(&self) -> PeerInfo {
        self.remote_peer_info.clone()
    }
}
