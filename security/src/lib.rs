use anyhow::{Error, Result};
use rnet_traits::stream::IReadWriteClose;

use crate::{conn::SecureConn, deffi_hellman::DHTransport};
pub mod conn;
pub mod deffi_hellman;

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

impl SecureTransport {
    pub fn new() -> Self {
        let sec_opts = vec![String::from("dh")];

        SecureTransport { sec_opts }
    }

    pub async fn secure_conn<T>(&self, stream: T, is_initiator: bool) -> Result<SecureConn<T>>
    where
        T: IReadWriteClose,
    {
        match is_initiator {
            true => return Ok(self.handshake_inbound(stream).await.unwrap()),
            false => return Ok(self.handshake_outbound(stream).await.unwrap()),
        }
    }

    pub async fn handshake_inbound<T>(&self, mut stream: T) -> Result<SecureConn<T>>
    where
        T: IReadWriteClose,
    {
        // Select on security transport
        // Via the security transport, conduct key exchange
        // TODO: Do this property as per sec-opts priority

        // For now the default security transport is deffi-heinman key exchange
        self.try_select(&mut stream, MULTISELECT_CONNECT, false)
            .await?;

        self.try_select(&mut stream, DEFFIE_HEINMAN, false).await?;

        // Inititate DEFFIE-HEINMAN shared-key exchange
        let dh_transport = DHTransport {};
        let cipher = dh_transport.handshake(&mut stream, false).await?;

        let secured_conn = SecureConn::new(cipher, stream);

        Ok(secured_conn)
    }

    pub async fn handshake_outbound<T>(&self, mut stream: T) -> Result<SecureConn<T>>
    where
        T: IReadWriteClose,
    {
        // For now the default security transport is deffi-heinman key exchange
        self.try_select(&mut stream, MULTISELECT_CONNECT, true)
            .await?;

        self.try_select(&mut stream, DEFFIE_HEINMAN, true).await?;

        // Inititate DEFFIE-HEINMAN shared-key exchange
        let dh_transport = DHTransport {};
        let cipher = dh_transport.handshake(&mut stream, true).await?;

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
}

#[cfg(test)]
mod tests {
    use chacha20poly1305::{
        aead::{Aead, OsRng},
        AeadCore, ChaCha20Poly1305, KeyInit, Nonce,
    };
    use x25519_dalek::PublicKey;

    #[test]
    fn test_diffie_heinman() {
        let csprng = OsRng {};
        let alice_secret = x25519_dalek::EphemeralSecret::random_from_rng(csprng);
        let bob_secret = x25519_dalek::EphemeralSecret::random_from_rng(csprng);

        let alice_public = PublicKey::from(&alice_secret);
        let bob_public = PublicKey::from(&bob_secret);

        let hex = hex::encode(alice_public.as_bytes());
        println!("{}", hex);

        let alice_public =
            PublicKey::from(<[u8; 32]>::try_from(alice_public.to_bytes().to_vec()).unwrap());

        let alice_shared_secret = alice_secret.diffie_hellman(&bob_public);
        let bob_shared_secret = bob_secret.diffie_hellman(&alice_public);

        let alice_key = chacha20poly1305::Key::from_slice(alice_shared_secret.as_bytes());
        let bob_key = chacha20poly1305::Key::from_slice(bob_shared_secret.as_bytes());

        let alice_cipher = ChaCha20Poly1305::new(alice_key);
        let bob_cipher = ChaCha20Poly1305::new(bob_key);

        let nonce = ChaCha20Poly1305::generate_nonce(&mut OsRng);
        let nonce_bytes: Vec<u8> = nonce.to_vec();
        println!("{:?}", nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = alice_cipher
            .encrypt(nonce, b"top secret orgy".as_ref())
            .unwrap();
        let plaintext = bob_cipher.decrypt(nonce, ciphertext.as_ref()).unwrap();

        println!("{:?}", ciphertext);
        println!("{:?}", plaintext);

        let ciphertext = bob_cipher
            .encrypt(nonce, b"top secret orgy haha".as_ref())
            .unwrap();
        let plaintext = alice_cipher.decrypt(nonce, ciphertext.as_ref()).unwrap();

        assert_eq!(&plaintext, b"top secret orgy haha");
    }
}
