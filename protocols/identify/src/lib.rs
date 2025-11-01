use anyhow::Result;
use rnet_peer::peer_info::PeerInfo;
use rnet_tcp::TcpConn;
use rnet_traits::transport::SendReceive;

pub async fn identify_seq(
    local_peer_info: &PeerInfo,
    stream: &mut TcpConn,
    is_initiator: bool,
) -> Result<PeerInfo> {
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
