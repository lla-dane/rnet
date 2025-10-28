use anyhow::{Error, Ok, Result};
use rnet_core::{IDENTIFY, MULTISELECT_CONNECT};
use rnet_identify::identify_seq;
use rnet_peer::PeerData;
use rnet_tcp::TcpConn;
use rnet_traits::transport::Connection;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::debug;

#[derive(Debug)]
pub struct Multiselect {}

impl Multiselect {
    pub async fn handshake(
        &self,
        stream: &mut TcpConn,
        peer_data: &Arc<Mutex<PeerData>>,
    ) -> Result<()> {
        // IDENTIFY HANDSHAKE

        self.try_select(stream, MULTISELECT_CONNECT)
            .await
            .expect("Multiselect handshake failed");

        self.try_select(stream, IDENTIFY)
            .await
            .expect("Identify handshake failed");

        debug!("Handshake complete");

        // Now run the IDENTIFY sequence
        identify_seq(peer_data, stream, false)
            .await
            .expect("Identify handshake failed");

        debug!("Identify handshake complete");

        Ok(())
    }

    pub async fn try_select(&self, stream: &mut TcpConn, proto: &str) -> Result<()> {
        stream.write(proto.as_bytes()).await?;

        let mut buf = [0u8; 32];
        let n = stream.read(&mut buf).await.unwrap();
        let received = String::from_utf8_lossy(&buf[..n]).to_string();

        if received.as_str() == proto {
            return Ok(());
        }

        return Err(Error::msg("Neogotiation failed"));
    }
}
