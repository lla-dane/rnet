use anyhow::Result;
use std::sync::Arc;
use tokio::sync::Mutex;

use rnet_peer::{peer_info::PeerInfo, PeerData};
use rnet_tcp::TcpConn;
use rnet_traits::transport::SendReceive;

pub async fn identify_seq(
    peer_data: &Arc<Mutex<PeerData>>,
    stream: &mut TcpConn,
    is_initiator: bool,
) -> Result<()> {
    let mut data = peer_data.lock().await;

    let peer_info_bytes = bincode::serialize(&data.peer_info.clone())?;
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

    // Insert the remote peer-info in the peer_store
    data.peer_store
        .insert(remote_peer_info.peer_id.clone(), remote_peer_info);

    Ok(())
}
