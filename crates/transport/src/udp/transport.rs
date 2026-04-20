use std::net::SocketAddr;

use anyhow::Result;
use async_trait::async_trait;
use identity::{multiaddr::Multiaddr, traits::transport::ITransport};
use tokio::net::UdpSocket;

use crate::udp::conn::UdpConn;

pub struct UdpTransport {
    listener: UdpSocket,
}

impl UdpTransport {
    pub fn get_local_addr(&self) -> Result<String> {
        let addr = self.listener.local_addr().unwrap().to_string();
        Ok(addr)
    }
}

#[async_trait]
impl ITransport<UdpConn> for UdpTransport {
    async fn listen(addr: &Multiaddr) -> Result<Self> {
        let local_ip = addr.value_for_protocol("ip4").unwrap();
        let port = addr.value_for_protocol("tcp").unwrap();
        let addr = format!("{}:{}", local_ip, port);

        let listener = UdpSocket::bind(addr).await?;
        Ok(Self { listener })
    }

    async fn accept(&self) -> Result<(UdpConn, SocketAddr)> {
        todo!()
    }

    async fn dial(addr: &Multiaddr) -> Result<UdpConn> {
        todo!()
    }
}

// maintain a map of peers in UDP transprot
// create an enum, new_conn and payload
// create a recv loop and start it in teh listen function 
// based on the enum of new_Conn send the packet to accept or the relevant udp_conn\
// maintain a similar write loop also, to receve write req form the internal api 
