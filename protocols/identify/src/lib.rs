use anyhow::{Ok, Result};
use rnet_peer::peer_info::PeerInfo;
use rnet_traits::transport::SendReceive;
use rnet_tcp::TcpConn;

pub async fn identify_seq(
    peer_info: &PeerInfo,
    stream: &mut TcpConn,
    is_initiator: bool,
    
) -> Result<()> {
    // Exchange the peer-info, non-initiator will send and
    // the initiator will wait for receive and then send its own

    let peer_info_bytes = bincode::serialize(peer_info)?;

    if !is_initiator {
        // Send the local peer-info
        stream.send_bytes(&peer_info_bytes).await?;

        // Now receive the remote peer-info
        let remote_peer_info_bytes = stream.recv_msg().await?;
        let remote_peer_info: PeerInfo = bincode::deserialize(&remote_peer_info_bytes)?;

        println!("Remote peer: {:?}", remote_peer_info);
    } else {
        // Receive peer-info
        let remote_peer_info_bytes = stream.recv_msg().await?;
        let remote_peer_info: PeerInfo = bincode::deserialize(&remote_peer_info_bytes)?;

        // Now send the local peer-info
        stream.send_bytes(&peer_info_bytes).await?;

        println!("Remote peer: {:?}", remote_peer_info);
    }

    Ok(())
}