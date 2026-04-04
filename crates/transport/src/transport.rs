use std::net::SocketAddr;

use anyhow::{Error, Result};
use identity::{
    multiaddr::Multiaddr,
    traits::{core::IReadWriteClose, transport::ITransport},
};

use crate::tcp::transport::TcpTransport;

pub enum TransportInner {
    Tcp(TcpTransport),
}

pub struct Transport {
    inner: TransportInner,
    listen_addr: String,
}

// new
// get_local_ip
// accept
impl Transport {
    pub async fn new(proto: &str, listen_addr: &mut Multiaddr) -> Result<Self> {
        match proto {
            "tcp" => {
                let listener = TcpTransport::listen(listen_addr).await.unwrap();

                Ok(Transport {
                    listen_addr: listener.get_local_addr().unwrap(),
                    inner: TransportInner::Tcp(listener),
                })
            }
            _ => Err(Error::msg("protocol not found")),
        }
    }

    pub fn get_local_ip(&self) -> Result<String> {
        Ok(self.listen_addr.clone())
    }

    pub async fn accept(&self) -> Result<(Box<dyn IReadWriteClose>, SocketAddr)> {
        match &self.inner {
            TransportInner::Tcp(tcp) => {
                let (stream, socket_addr) = tcp.accept().await.unwrap();
                Ok((Box::new(stream), socket_addr))
            }
        }
    }
}
