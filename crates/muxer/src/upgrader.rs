use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use identity::peer::PeerInfo;
use identity::traits::core::IRawConnection;
use tokio::sync::mpsc::Sender;
use tokio::sync::Mutex;

use crate::conn::MuxedConn;
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
        handlers: Arc<Mutex<HashMap<String, AsyncHandler>>>,
        global_event_tx: Sender<Vec<u8>>,
    ) -> Result<(MuxedConn, Sender<Vec<u8>>)>
    where
        T: IRawConnection + Send + Sync + 'static,
    {
        Ok(self
            .transport
            .handshake(stream, is_initiator, remote_peer, handlers, global_event_tx)
            .await
            .unwrap())
    }
}
