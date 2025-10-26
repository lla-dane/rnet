use std::net::SocketAddr;

use anyhow::{Ok, Result};
use async_trait::async_trait;
use rnet_core::{Connection, Transport};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

pub struct TcpConn {
    stream: TcpStream,
}

#[async_trait]
impl Connection for TcpConn {
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let n = self.stream.read(buf).await?;
        Ok(n)
    }

    async fn write(&mut self, data: &[u8]) -> Result<usize> {
        let n = self.stream.write(data).await?;
        Ok(n)
    }

    async fn close(&mut self) -> Result<()> {
        self.stream.shutdown().await?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct TcpTransport {
    listener: TcpListener,
}

#[async_trait]
impl Transport for TcpTransport {
    type Conn = TcpConn;

    async fn listen(addr: &str) -> Result<Self> {
        let listener = TcpListener::bind(addr).await?;
        Ok(Self { listener })
    }

    async fn accept(&mut self) -> Result<(Self::Conn, SocketAddr)> {
        let (stream, addr) = self.listener.accept().await?;
        Ok((TcpConn { stream }, addr))
    }

    async fn dial(addr: &str) -> Result<Self::Conn> {
        let stream = TcpStream::connect(addr).await?;
        Ok(TcpConn { stream })
    }
}
