use std::collections::HashMap;

use anyhow::Result;
use peer::peer_info::PeerInfo;
use traits::{core::IRawConnection, muxer::IMuxedConn};
use tokio::sync::mpsc::Sender;

use crate::{mplex::conn::AsyncHandler, transport::MuxerTransport};

pub struct MuxerUpgrader {
    transport: MuxerTransport,
}

impl Default for MuxerUpgrader {
    fn default() -> Self {
        Self::new()
    }
}

impl MuxerUpgrader {
    pub fn new() -> Self {
        MuxerUpgrader {
            transport: MuxerTransport::new(),
        }
    }

    pub async fn update<T>(
        &self,
        stream: T,
        is_initiator: bool,
        remote_peer: PeerInfo,
        handlers: HashMap<String, AsyncHandler>,
        global_event_tx: Sender<Vec<u8>>,
    ) -> Result<(impl IMuxedConn, Sender<Vec<u8>>)>
    where
        T: IRawConnection + Send + Sync,
    {
        Ok(self
            .transport
            .handshake(stream, is_initiator, remote_peer, handlers, global_event_tx)
            .await
            .unwrap())
    }
}
