use std::{collections::HashMap, sync::Arc};

use anyhow::Result;
use rnet_host::RawConnection;
use rnet_peer::peer_info::PeerInfo;
use tokio::sync::Mutex;

use crate::mplex_stream::MuxedStream;

pub struct MuxedConn {
    raw_conn: Arc<Mutex<RawConnection>>,
    pub remote_peer_info: PeerInfo,
    pub is_initiator: bool,
    streams: HashMap<u32, MuxedStream>,
}

impl MuxedConn {
    pub fn new(conn: RawConnection, is_initiator: bool) -> Self {
        let peer_info = conn.peer_info();

        Self {
            raw_conn: Arc::new(Mutex::new(conn)),
            remote_peer_info: peer_info,
            is_initiator,
            streams: HashMap::new(),
        }
    }
    pub async fn read(&mut self) -> Result<()> {
        let frames = {
            let mut raw_conn = self.raw_conn.lock().await;
            raw_conn.read().await?
        };

        // Process frames, to see the following:
        // - create a new muxed-stream as per the headers
        // - see which stream the payload belongs to, as per the headers

        Ok(())
    }

    pub async fn write(&mut self, msg: &Vec<u8>) -> Result<()> {
        {
            let mut raw_conn = self.raw_conn.lock().await;
            raw_conn.write(msg).await?
        }

        Ok(())
    }

    pub async fn close(&mut self) -> Result<()> {
        {
            let mut raw_conn = self.raw_conn.lock().await;
            raw_conn.close().await?
        }

        Ok(())
    }
}
