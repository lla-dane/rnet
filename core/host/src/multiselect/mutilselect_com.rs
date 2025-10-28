use anyhow::{Error, Ok, Result};
use rnet_core::{IDENTIFY, MULTISELECT_CONNECT};
use rnet_identify::identify_seq;
use rnet_peer::peer_info::PeerInfo;
use rnet_tcp::TcpConn;
use rnet_traits::transport::Connection;
use tracing::debug;

#[derive(Debug)]
pub struct MultiselectComm {}

impl MultiselectComm {
    pub async fn handshake(&self, stream: &mut TcpConn, peer_info: &PeerInfo) -> Result<()> {
        // IDENTIFY HANDSHAKE
        self.try_select(stream, MULTISELECT_CONNECT)
            .await
            .expect("Multiselect handshake failed");

        self.try_select(stream, IDENTIFY)
            .await
            .expect("Identify handshake failed");

        debug!("Handshake complete");

        // Now run the IDENTIFY sequence
        identify_seq(peer_info, stream, true)
            .await
            .expect("Identify handshake failed");

        Ok(())
    }

    pub async fn try_select(&self, stream: &mut TcpConn, proto: &str) -> Result<()> {
        let mut buf = [0u8; 32];
        let n = stream.read(&mut buf).await.unwrap();
        let received = String::from_utf8_lossy(&buf[..n]).to_string();

        if received.as_str() == proto {
            stream.write(proto.as_bytes()).await.unwrap();
            return Ok(());
        }

        return Err(Error::msg("Negotiation failed"));
    }
}
