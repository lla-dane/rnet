use std::net::SocketAddr;

use anyhow::{Error, Result};
use identity::{
    multiaddr::Multiaddr,
    traits::{core::IReadWriteClose, transport::ITransport},
};
use tokio::net::TcpStream;

use crate::tcp::{conn::TcpConn, transport::TcpTransport};

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
    pub async fn new(proto: &str, listen_addr: &mut Multiaddr) -> Result<(Self, String)> {
        match proto {
            "tcp" => {
                let listener = TcpTransport::listen(listen_addr).await.unwrap();
                let listen_addr = listener.get_local_addr().unwrap();

                Ok((
                    Transport {
                        listen_addr: listen_addr.clone(),
                        inner: TransportInner::Tcp(listener),
                    },
                    listen_addr,
                ))
            }
            _ => Err(Error::msg("protocol not found")),
        }
    }

    pub async fn accept(&self) -> Result<(Box<dyn IReadWriteClose>, SocketAddr)> {
        match &self.inner {
            TransportInner::Tcp(tcp) => {
                let (stream, socket_addr) = tcp.accept().await.unwrap();
                Ok((Box::new(stream), socket_addr))
            }
        }
    }

    pub async fn dial(&self, addr: &Multiaddr) -> Result<Box<dyn IReadWriteClose>> {
        let local_ip = addr.value_for_protocol("ip4").unwrap();
        let port = addr.value_for_protocol("tcp").unwrap();
        let addr = format!("{}:{}", local_ip, port);

        match &self.inner {
            TransportInner::Tcp(_) => {
                let stream = TcpStream::connect(addr).await.unwrap();
                Ok(Box::new(TcpConn { stream }))
            }
        }
    }

    pub fn get_local_ip(&self) -> Result<String> {
        Ok(self.listen_addr.clone())
    }
}
