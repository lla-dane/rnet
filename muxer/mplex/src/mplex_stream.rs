use std::sync::Arc;

use anyhow::Result;
use rnet_host::RawConnection;
use rnet_peer::peer_info::PeerInfo;
use tokio::sync::Mutex;

pub struct MuxedStream {
    raw_conn: Arc<Mutex<RawConnection>>,
    stream_id: u32,
    is_initiator: bool,
    pub remote_peer_info: PeerInfo,
}

impl MuxedStream {
    pub fn new(
        conn: Arc<Mutex<RawConnection>>,
        stream_id: u32,
        is_initiator: bool,
        remote_peer_info: PeerInfo,
    ) -> Self {
        Self {
            raw_conn: conn,
            stream_id,
            is_initiator: is_initiator,
            remote_peer_info,
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

    pub fn peer_info(&self) -> PeerInfo {
        self.remote_peer_info.clone()
    }
}
