use std::collections::HashMap;

use anyhow::{Error, Result};
use rnet_core::MULTISELECT_CONNECT;
use rnet_peer::peer_info::PeerInfo;
use rnet_traits::{core::IRawConnection, muxer::IMuxedConn};
use tokio::sync::mpsc::{self, Sender};

use crate::mplex::conn::{AsyncHandler, MplexConn, MPLEX};

pub struct MuxerTransport {
    pub muxer_opts: Vec<String>,
}

impl Default for MuxerTransport {
    fn default() -> Self {
        Self::new()
    }
}

/// mux_conn
/// handshake_inbound
/// handshake_outbound
/// try_select
/// select_transport
impl MuxerTransport {
    pub fn new() -> Self {
        let muxer_opts = vec![String::from("mplex")];

        MuxerTransport { muxer_opts }
    }

    pub async fn mux_conn<T>(
        &self,
        stream: T,
        is_initiator: bool,
        remote_peer: PeerInfo,
        handlers: HashMap<String, AsyncHandler>,
    ) -> Result<(impl IMuxedConn, Sender<Vec<u8>>)>
    where
        T: IRawConnection + Send + Sync,
    {
        Ok(self
            .handshake(stream, is_initiator, remote_peer, handlers)
            .await
            .unwrap())
    }

    pub async fn handshake<T>(
        &self,
        mut stream: T,
        is_initiator: bool,
        remote_peer: PeerInfo,
        handlers: HashMap<String, AsyncHandler>,
    ) -> Result<(impl IMuxedConn, Sender<Vec<u8>>)>
    where
        T: IRawConnection + Send + Sync,
    {
        // Select on the protocol to use
        // TODO: Do this properly: muxer-opts priority

        self.try_select(&mut stream, MULTISELECT_CONNECT, is_initiator)
            .await
            .unwrap();

        self.try_select(&mut stream, MPLEX, is_initiator)
            .await
            .unwrap();

        let (muxed_conn_mpsc_tx, muxed_conn_mpsc_rx) = mpsc::channel::<Vec<u8>>(100);
        let muxed_conn = MplexConn::new(
            stream,
            is_initiator,
            remote_peer,
            handlers,
            muxed_conn_mpsc_tx.clone(),
            muxed_conn_mpsc_rx,
        );

        Ok((muxed_conn, muxed_conn_mpsc_tx))
    }

    async fn try_select<T>(&self, stream: &mut T, proto: &str, is_initiator: bool) -> Result<()>
    where
        T: IRawConnection,
    {
        match is_initiator {
            true => {
                let proto_bytes = bincode::serialize(&proto)?;
                stream.write(&proto_bytes).await?;

                let msg_bytes = stream.read().await.unwrap();
                let received: String = bincode::deserialize(&msg_bytes)?;

                if received.as_str() == proto {
                    return Ok(());
                }
            }

            false => {
                let msg_bytes = stream.read().await.unwrap();
                let received: String = bincode::deserialize(&msg_bytes)?;

                if received.as_str() == proto {
                    let proto_bytes = bincode::serialize(&proto)?;
                    stream.write(&proto_bytes).await.unwrap();
                    return Ok(());
                }
            }
        }

        Err(Error::msg("Negotiation failed"))
    }

    pub fn select_transport(&self) -> Result<()> {
        Ok(())
    }
}
