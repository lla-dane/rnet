use anyhow::{Error, Ok, Result};
use async_trait::async_trait;
use rnet_core::{IDENTIFY, MULTISELECT_CONNECT};
use rnet_peer::peer_info::PeerInfo;
use rnet_traits::{core::IMultistream, security::ISecuredConn};
use rnet_transport::raw_conn::RawConnection;

#[derive(Debug)]
pub struct Multiselect {}

#[async_trait]
impl<T> IMultistream<T, RawConnection<T>, PeerInfo> for Multiselect
where
    T: ISecuredConn + Send + 'static,
{
    async fn handshake(
        &self,
        local_peer_info: &PeerInfo,
        mut stream: T,
        is_initiator: bool,
    ) -> Result<RawConnection<T>> {
        self.try_select(&mut stream, MULTISELECT_CONNECT, is_initiator)
            .await
            .expect("Multistream handshake failed");

        self.try_select(&mut stream, IDENTIFY, is_initiator)
            .await
            .expect("Identify handshake failed");

        let remote = self
            .identify(local_peer_info, &mut stream, is_initiator)
            .await
            .expect("Identify handshake failed");

        Ok(RawConnection {
            stream,
            peer_info: remote,
            is_initiator: false,
        })
    }
    async fn try_select(&self, stream: &mut T, proto: &str, is_initiator: bool) -> Result<()> {
        match is_initiator {
            true => {
                let proto_bytes = bincode::serialize(&proto)?;
                stream.write(&proto_bytes).await?;

                let msg_bytes = stream.read().await.unwrap();
                let received: String = bincode::deserialize(&msg_bytes)?;

                if received.as_str() == proto {
                    return Ok(());
                }
            }

            false => {
                let msg_bytes = stream.read().await.unwrap();
                let received: String = bincode::deserialize(&msg_bytes)?;

                if received.as_str() == proto {
                    let proto_bytes = bincode::serialize(&proto)?;
                    stream.write(&proto_bytes).await.unwrap();
                    return Ok(());
                }
            }
        }

        Err(Error::msg("Negotiation failed"))
    }

    async fn identify(
        &self,
        local_peer_info: &PeerInfo,
        stream: &mut T,
        is_initiator: bool,
    ) -> Result<PeerInfo>
    where
        T: ISecuredConn,
    {
        let peer_info_bytes = bincode::serialize(local_peer_info)?;
        let remote_peer_info: PeerInfo;

        match is_initiator {
            false => {
                stream.write(&peer_info_bytes).await?;

                let remote_peer_info_bytes = stream.read().await.unwrap();
                remote_peer_info = bincode::deserialize(&remote_peer_info_bytes)?;
            }
            true => {
                let remote_peer_info_bytes = stream.read().await.unwrap();
                remote_peer_info = bincode::deserialize(&remote_peer_info_bytes)?;

                stream.write(&peer_info_bytes).await?;
            }
        }
        Ok(remote_peer_info)
    }
}
