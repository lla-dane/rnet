use anyhow::Result;
use async_trait::async_trait;
use identity::{multiaddr::Multiaddr, traits::transport::ITransport};
use std::net::SocketAddr;
use tokio::net::{TcpListener, TcpStream};

use crate::tcp::conn::TcpConn;

#[derive(Debug)]
pub struct TcpTransport {
    listener: TcpListener,
}

impl TcpTransport {
    pub fn get_local_addr(&self) -> Result<String> {
        let addr = self.listener.local_addr().unwrap().to_string();
        Ok(addr)
    }
}

#[async_trait]
impl ITransport<TcpConn> for TcpTransport {
    async fn listen(addr: &Multiaddr) -> Result<Self> {
        let local_ip = addr.value_for_protocol("ip4").unwrap();
        let port = addr.value_for_protocol("tcp").unwrap();
        let addr = format!("{}:{}", local_ip, port);

        let listener = TcpListener::bind(addr).await?;
        Ok(Self { listener })
    }

    async fn accept(&self) -> Result<(TcpConn, SocketAddr)> {
        let (stream, addr) = self.listener.accept().await?;
        stream.set_nodelay(true).unwrap();
        Ok((TcpConn { stream }, addr))
    }

    async fn dial(&self, addr: &Multiaddr) -> Result<TcpConn> {
        let local_ip = addr.value_for_protocol("ip4").unwrap();
        let port = addr.value_for_protocol("tcp").unwrap();
        let addr = format!("{}:{}", local_ip, port);

        let stream = TcpStream::connect(addr).await?;
        stream.set_nodelay(true).unwrap();
        Ok(TcpConn { stream })
    }
}
