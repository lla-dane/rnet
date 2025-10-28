use anyhow::{bail, Ok, Result};
use async_trait::async_trait;
use rnet_multiaddr::Multiaddr;
use rnet_traits::transport::{Connection, SendReceive, Transport};
use std::net::SocketAddr;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

#[derive(Debug)]
pub struct TcpTransport {
    listener: TcpListener,
}

pub struct TcpConn {
    stream: TcpStream,
}

#[async_trait]
impl Connection for TcpConn {
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let n = self.stream.read(buf).await?;
        return Ok(n);
    }
    async fn write(&mut self, buf: &[u8]) -> Result<usize> {
        let n = self.stream.write(buf).await?;
        return Ok(n);
    }
    async fn close(&mut self) -> Result<()> {
        self.stream.shutdown().await?;
        Ok(())
    }
}

#[async_trait]
impl SendReceive for TcpConn {
    async fn read_exact(&mut self, buf: &mut [u8]) -> Result<()> {
        let mut total = 0;
        while total < buf.len() {
            let n = self.read(&mut buf[total..]).await?;
            if n == 0 {
                bail!("Connection closed while reading");
            }
            total += n;
        }

        Ok(())
    }

    async fn recv_msg(&mut self) -> Result<Vec<u8>> {
        let mut len_buf = [0u8; 4];
        self.read_exact(&mut len_buf).await?;
        let len = u32::from_be_bytes(len_buf) as usize;

        let mut data = vec![0u8; len];
        self.read_exact(&mut data).await?;

        Ok(data)
    }

    async fn send_bytes(&mut self, msg: &Vec<u8>) -> Result<()> {
        let len = msg.len() as u32;
        let len_bytes = len.to_be_bytes();

        self.write(&len_bytes).await?;
        self.write(msg).await?;

        Ok(())
    }
}

impl TcpConn {
    pub fn get_ip(&self) -> Result<SocketAddr> {
        Ok(self.stream.local_addr().unwrap())
    }
}

impl TcpTransport {
    pub fn get_local_addr(&self) -> Result<String> {
        let addr = self.listener.local_addr().unwrap().to_string();
        Ok(addr)
    }
}

#[async_trait]
impl Transport for TcpTransport {
    type Conn = TcpConn;

    async fn listen(addr: &Multiaddr) -> Result<Self> {
        let local_ip = addr.value_for_protocol("ip4").unwrap();
        let port = addr.value_for_protocol("tcp").unwrap();
        let addr = format!("{}:{}", local_ip, port);

        let listener = TcpListener::bind(addr).await?;
        Ok(Self { listener: listener })
    }

    async fn accept(&self) -> Result<(Self::Conn, SocketAddr)> {
        let (stream, addr) = self.listener.accept().await?;
        Ok((TcpConn { stream }, addr))
    }

    async fn dial(addr: &Multiaddr) -> Result<Self::Conn> {
        let local_ip = addr.value_for_protocol("ip4").unwrap();
        let port = addr.value_for_protocol("tcp").unwrap();
        let addr = format!("{}:{}", local_ip, port);

        let stream = TcpStream::connect(addr).await?;
        Ok(TcpConn { stream })
    }
}
