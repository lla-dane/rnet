use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

use anyhow::{Error, Result};
use async_trait::async_trait;
use identity::traits::core::IReadWriteClose;
use tokio::sync::mpsc::{Receiver, Sender};

pub struct UdpConn {
    socket_mspc_tx: Sender<Vec<u8>>,
    socket_mpsc_rx: Receiver<Vec<u8>>,
    peer: SocketAddr,
}

#[async_trait]
impl IReadWriteClose for UdpConn {
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let payload = self.socket_mpsc_rx.recv().await.unwrap();
        buf[0..payload.len()].copy_from_slice(&payload);

        Ok(payload.len())
    }

    async fn write(&mut self, buf: &[u8]) -> Result<()> {
        let mut buffer = buf.to_vec();

        append_addr(self.peer, &mut buffer).unwrap();
        self.socket_mspc_tx.send(buffer).await.unwrap();
        Ok(())
    }

    async fn close(&mut self) -> Result<()> {
        Ok(())
    }

    async fn read_exact(&mut self, buf: &mut [u8]) -> Result<()> {
        Ok(())
    }
    async fn recv_msg(&mut self) -> Result<Vec<u8>> {
        todo!()
    }

    async fn send_bytes(&mut self, msg: &Vec<u8>) -> Result<()> {
        Ok(())
    }
}

pub fn append_addr(peer_addr: SocketAddr, buf: &mut Vec<u8>) -> Result<()> {
    let addr = match peer_addr {
        SocketAddr::V4(v4) => v4,
        _ => panic!("Only IPv4 supported"),
    };

    let ip = addr.ip().octets(); // [u8; 4]
    let port = addr.port().to_be_bytes(); // [u8; 2]

    buf.extend_from_slice(&ip);
    buf.extend_from_slice(&port);

    Ok(())
}

pub fn extract_addr(buf: &[u8]) -> Result<(Vec<u8>, SocketAddrV4)> {
    if buf.len() < 6 {
        return Err(Error::msg("buffer too small to contain addr"));
    }

    let payload_len = buf.len() - 6;
    let ip_bytes: [u8; 4] = buf[payload_len..payload_len + 4].try_into().unwrap();
    let port_bytes: [u8; 2] = buf[payload_len + 4..payload_len + 6].try_into().unwrap();

    let ip = Ipv4Addr::from(ip_bytes);
    let port = u16::from_be_bytes(port_bytes);

    let addr = SocketAddrV4::new(ip, port);
    let payload = buf[..payload_len].to_vec();

    Ok((payload, addr))
}
