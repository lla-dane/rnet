use std::collections::HashMap;

use anyhow::Result;
use identity::peer::PeerInfo;
use identity::traits::{
    core::{IRawConnection, IReadWriteClose},
    muxer::IMuxedConn,
};
use muxer::{mplex::conn::AsyncHandler, upgrader::MuxerUpgrader};
use security::{conn::SecureConn, upgrader::SecurityUpgrader};
use tokio::sync::mpsc::Sender;

pub struct ConnUpgrader {
    sec_upgrader: SecurityUpgrader,
    mux_upgrader: MuxerUpgrader,
}

impl Default for ConnUpgrader {
    fn default() -> Self {
        Self::new()
    }
}

impl ConnUpgrader {
    pub fn new() -> Self {
        ConnUpgrader {
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
        global_event_tx: Sender<Vec<u8>>,
    ) -> Result<(impl IMuxedConn, Sender<Vec<u8>>)>
    where
        T: IRawConnection + Send + Sync,
    {
        Ok(self
            .mux_upgrader
            .update(stream, is_initiator, remote_peer, handlers, global_event_tx)
            .await
            .unwrap())
    }
}
