use anyhow::{Error, Ok, Result};
use rnet_core::{IDENTIFY, MULTISELECT_CONNECT};
use rnet_identify::identify_seq;
use rnet_peer::peer_info::PeerInfo;
use rnet_traits::stream::IReadWriteClose;
use rnet_transport::RawConnection;

#[derive(Debug)]
pub struct MultiselectComm {}

impl MultiselectComm {
    pub async fn handshake<T>(
        &self,
        local_peer_info: &PeerInfo,
        mut stream: T,
    ) -> Result<RawConnection<T>>
    where
        T: IReadWriteClose,
    {
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

        Ok(RawConnection {
            stream,
            peer_info,
            is_initiator: false,
        })
    }

    pub async fn try_select<T>(&self, stream: &mut T, proto: &str) -> Result<()>
    where
        T: IReadWriteClose,
    {
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
