use std::collections::HashMap;

use anyhow::Result;
use rnet_muxer::{mplex::conn::AsyncHandler, upgrader::MuxerUpgrader};
use rnet_peer::peer_info::PeerInfo;
use rnet_security::{conn::SecureConn, upgrader::SecurityUpgrader};
use rnet_traits::{
    conn::{IMuxedConn, IRawConnection},
    stream::IReadWriteClose,
};
use tokio::sync::mpsc::Sender;

pub struct TransportUpgrader {
    sec_upgrader: SecurityUpgrader,
    mux_upgrader: MuxerUpgrader,
}

impl Default for TransportUpgrader {
    fn default() -> Self {
        Self::new()
    }
}

impl TransportUpgrader {
    pub fn new() -> Self {
        TransportUpgrader {
            sec_upgrader: SecurityUpgrader::new(),
            mux_upgrader: MuxerUpgrader::new(),
        }
    }

    pub async fn update_security<T>(&self, stream: T, is_initiator: bool) -> Result<SecureConn<T>>
    where
        T: IReadWriteClose + Send + Sync,
    {
        Ok(self
            .sec_upgrader
            .update(stream, is_initiator)
            .await
            .unwrap())
    }

    pub async fn update_muxer<T>(
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
            .mux_upgrader
            .update(stream, is_initiator, remote_peer, handlers)
            .await
            .unwrap())
    }
}
