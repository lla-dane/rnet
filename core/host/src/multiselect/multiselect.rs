use anyhow::{Ok, Result};
use rnet_core::MULTISELECT_CONNECT;
use rnet_core_traits::transport::Connection;
use rnet_tcp::TcpConn;
use tracing::debug;

#[derive(Debug)]
pub struct Multiselect {}

impl Multiselect {
    pub async fn handshake(&self, stream: &mut TcpConn) -> Result<()> {
        // Multiselect
        if self.try_select(stream, MULTISELECT_CONNECT).await.unwrap() {
            debug!("Handshake complete");
        }

        Ok(())
    }

    pub async fn try_select(&self, stream: &mut TcpConn, proto: &str) -> Result<bool> {
        stream.write(proto.as_bytes()).await?;

        let mut buf = [0u8; 32];
        let n = stream.read(&mut buf).await.unwrap();
        let received = String::from_utf8_lossy(&buf[..n]).to_string();

        if received.as_str() == proto {
            return Ok(true);
        }

        Ok(false)
    }
}
