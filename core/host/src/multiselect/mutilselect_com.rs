use anyhow::{Error, Ok, Result};
use rnet_core::{IDENTIFY, MULTISELECT_CONNECT};
use rnet_identify::identify_seq;
use rnet_peer::peer_info::PeerInfo;
use rnet_tcp::TcpConn;
use rnet_traits::transport::SendReceive;
use rnet_transport::RawConnection;
use tracing::debug;

#[derive(Debug)]
pub struct MultiselectComm {}

impl MultiselectComm {
    pub async fn handshake(
        &self,
        local_peer_info: &PeerInfo,
        mut stream: TcpConn,
    ) -> Result<RawConnection> {
        // IDENTIFY HANDSHAKE
        self.try_select(&mut stream, MULTISELECT_CONNECT)
            .await
            .expect("Multiselect handshake failed");

        self.try_select(&mut stream, IDENTIFY)
            .await
            .expect("Identify handshake failed");

        // Now run the IDENTIFY sequence
        let peer_info = identify_seq(local_peer_info, &mut stream, true)
            .await
            .expect("Identify handshake failed");

        debug!("Identify handshake complete");

        Ok(RawConnection {
            stream,
            peer_info,
            is_initiator: false,
        })
    }

    pub async fn try_select(&self, stream: &mut TcpConn, proto: &str) -> Result<()> {
        let msg_bytes = stream.recv_msg().await.unwrap();
        let received: String = bincode::deserialize(&msg_bytes)?;

        if received.as_str() == proto {
            let proto_bytes = bincode::serialize(&proto)?;
            stream.send_bytes(&proto_bytes).await.unwrap();
            return Ok(());
        }

        return Err(Error::msg("Negotiation failed"));
    }
}
