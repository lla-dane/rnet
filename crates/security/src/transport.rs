use anyhow::{Error, Result};
use identity::traits::core::IReadWriteClose;

use crate::{conn::SecureConn, deffi_hellman::DHTransport};

pub const MULTISELECT_CONNECT: &str = "mutilselect/0.0.1";
pub const DEFFIE_HEINMAN: &str = "rnet/deffi_heinman/0.0.1";

pub struct SecureTransport {
    pub sec_opts: Vec<String>,
}

impl Default for SecureTransport {
    fn default() -> Self {
        Self::new()
    }
}

/// secure_conn
/// handshake_inbound
/// handshake_outbound
/// try_select
/// select_transport
impl SecureTransport {
    pub fn new() -> Self {
        let sec_opts = vec![String::from("dh")];

        SecureTransport { sec_opts }
    }

    pub async fn secure_conn<T>(&self, stream: T, is_initiator: bool) -> Result<SecureConn<T>>
    where
        T: IReadWriteClose,
    {
        Ok(self.handshake(stream, is_initiator).await.unwrap())
    }

    pub async fn handshake<T>(&self, mut stream: T, is_initiator: bool) -> Result<SecureConn<T>>
    where
        T: IReadWriteClose,
    {
        // Select on security transport
        // Via the security transport, conduct key exchange
        // TODO: Do this property as per sec-opts priority

        // For now the default security transport is deffi-heinman key exchange
        self.try_select(&mut stream, MULTISELECT_CONNECT, is_initiator)
            .await?;

        self.try_select(&mut stream, DEFFIE_HEINMAN, is_initiator)
            .await?;

        // Inititate DEFFIE-HEINMAN shared-key exchange
        let dh_transport = DHTransport {};
        let cipher = dh_transport.handshake(&mut stream, is_initiator).await?;

        let secured_conn = SecureConn::new(cipher, stream);

        Ok(secured_conn)
    }

    async fn try_select<T>(&self, stream: &mut T, protocol: &str, is_initiator: bool) -> Result<()>
    where
        T: IReadWriteClose,
    {
        match is_initiator {
            true => {
                let proto_bytes = bincode::serialize(&protocol)?;
                stream.send_bytes(&proto_bytes).await?;

                let response = stream.recv_msg().await.unwrap();
                let proto_resp: String = bincode::deserialize(&response)?;

                if proto_resp.as_str() == protocol {
                    return Ok(());
                }
            }
            false => {
                let msg_bytes = stream.recv_msg().await.unwrap();
                let received: String = bincode::deserialize(&msg_bytes)?;

                if received.as_str() == protocol {
                    let proto_bytes = bincode::serialize(&protocol)?;
                    stream.send_bytes(&proto_bytes).await.unwrap();

                    return Ok(());
                }
            }
        };

        Err(Error::msg("Negotiation failed"))
    }

    pub fn select_transport(&self) -> Result<()> {
        Ok(())
    }
}
