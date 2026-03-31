use std::collections::HashMap;

use anyhow::{Error, Ok, Result};
use async_trait::async_trait;
use rnet_peer::peer_info::PeerInfo;
use rnet_traits::stream::IMuxedStream;
use tokio::sync::mpsc::{Receiver, Sender};

use crate::mplex::{
    conn::AsyncHandler,
    headers::{build_frame, MuxedStreamFlag},
};

pub struct MplexStream {
    muxed_conn_mpsc_tx: Sender<Vec<u8>>,
    muxed_stream_mpsc_rx: Receiver<Vec<u8>>,
    pub stream_id: u32,
    pub is_initiator: bool,
    pub remote_peer_info: PeerInfo,
    pub handlers: HashMap<String, AsyncHandler>,
}

impl MplexStream {
    pub fn new(
        muxed_conn_mpsc_tx: Sender<Vec<u8>>,
        muxed_stream_mpsc_rx: Receiver<Vec<u8>>,
        stream_id: u32,
        is_initiator: bool,
        remote_peer_info: PeerInfo,
        handlers: HashMap<String, AsyncHandler>,
    ) -> MplexStream {
        MplexStream {
            muxed_conn_mpsc_tx,
            muxed_stream_mpsc_rx,
            stream_id,
            is_initiator,
            remote_peer_info,
            handlers,
        }
    }

    pub fn peer_info(&self) -> PeerInfo {
        self.remote_peer_info.clone()
    }
}

#[async_trait]
impl IMuxedStream for MplexStream {
    async fn write(&self, msg: &Vec<u8>) -> Result<()> {
        if self.is_initiator {
            let frame = build_frame(self.stream_id, MuxedStreamFlag::MessageRequest, msg);
            self.muxed_conn_mpsc_tx.send(frame).await?;
        } else {
            let frame = build_frame(self.stream_id, MuxedStreamFlag::MessageResponse, msg);
            self.muxed_conn_mpsc_tx.send(frame).await?;
        }

        Ok(())
    }

    async fn read(&mut self) -> Result<Vec<u8>> {
        match self.muxed_stream_mpsc_rx.recv().await {
            Some(payload) => return Ok(payload),
            None => return Err(Error::msg("mpsc receiver down")),
        }
    }

    async fn server_handshake(mut self) -> Result<()> {
        let payload = self.read().await.unwrap();
        let protocol = String::from_utf8(payload.clone()).unwrap();

        let protocols: Vec<String> = self.handlers.keys().cloned().collect();

        if !protocols.contains(&protocol) {
            return Err(Error::msg("Server: Protocol negotiation failed"));
        }

        let frame = build_frame(self.stream_id, MuxedStreamFlag::HandshakeRes, &payload);
        self.muxed_conn_mpsc_tx.send(frame).await?;

        let handler = self
            .handlers
            .get(&protocol)
            .cloned()
            .ok_or_else(|| Error::msg("Protocol not found"))?;

        handler(self).await.unwrap();
        Ok(())
    }

    async fn client_handshake(mut self, protocol: Vec<u8>) -> Result<()> {
        let frame = build_frame(self.stream_id, MuxedStreamFlag::HandshakeReq, &protocol);
        self.muxed_conn_mpsc_tx.send(frame).await?;

        let res = self.read().await.unwrap();
        if protocol != res {
            return Err(Error::msg("Client: Protocol negotiation failed"));
        }

        let protocol = String::from_utf8(protocol).unwrap();
        let handler = self
            .handlers
            .get(&protocol)
            .cloned()
            .ok_or_else(|| Error::msg("Protocol not found"))?;

        handler(self).await.unwrap();

        Ok(())
    }
}
