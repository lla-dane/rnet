use anyhow::Result;
use rnet_peer::peer_info::PeerInfo;
use rnet_traits::stream::IReadWriteClose;

pub async fn identify_seq<T>(
    local_peer_info: &PeerInfo,
    stream: &mut T,
    is_initiator: bool,
) -> Result<PeerInfo>
where
    T: IReadWriteClose,
{
    let peer_info_bytes = bincode::serialize(local_peer_info)?;
    let remote_peer_info: PeerInfo;

    if !is_initiator {
        // we send first, then receive
        stream.send_bytes(&peer_info_bytes).await?;

        let remote_peer_info_bytes = stream.recv_msg().await?;
        remote_peer_info = bincode::deserialize(&remote_peer_info_bytes)?;
    } else {
        // we receive first, then send
        let remote_peer_info_bytes = stream.recv_msg().await?;
        remote_peer_info = bincode::deserialize(&remote_peer_info_bytes)?;

        stream.send_bytes(&peer_info_bytes).await?;
    }

    Ok(remote_peer_info)
}
