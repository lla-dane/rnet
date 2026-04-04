use anyhow::{Error, Result};
use async_trait::async_trait;

use chacha20poly1305::ChaCha20Poly1305;
use traits::{core::IReadWriteClose, security::ISecuredConn};

use crate::ISecureCipher;

pub const NONCE_LEN: usize = 12;

pub struct SecureConn<T>
where
    T: IReadWriteClose,
{
    cipher: ChaCha20Poly1305,
    stream: T,
}

impl<T> SecureConn<T>
where
    T: IReadWriteClose,
{
    pub fn new(cipher: ChaCha20Poly1305, stream: T) -> Self {
        Self { cipher, stream }
    }
}

#[async_trait]
impl<T> ISecuredConn for SecureConn<T>
where
    T: IReadWriteClose + Send,
{
    async fn read(&mut self) -> Result<Vec<u8>> {
        // Receive the msg
        // separate the nonce - 12 bytes
        // decrypt the msg
        // pass on the vec<u8>
        let bytes = self.stream.recv_msg().await?;
        if bytes.len() < NONCE_LEN {
            return Err(Error::msg("message too short"));
        }

        let (nonce_bytes, ciphertext) = bytes.split_at(NONCE_LEN);

        let plaintext = self.cipher.idecrypt(nonce_bytes, ciphertext).unwrap();
        Ok(plaintext)
    }

    async fn write(&mut self, msg: &Vec<u8>) -> Result<()> {
        // Generate a nonce
        // encrypt the plaintext
        // Insert the nonce - 12 bytes in front
        // send off the paylaod

        let (ciphertext, nonce) = self.cipher.iencrypt(msg).unwrap();

        let mut payload = Vec::with_capacity(nonce.len() + ciphertext.len());

        payload.extend_from_slice(nonce.as_slice()); // 12 bytes
        payload.extend_from_slice(&ciphertext);

        Ok(self.stream.send_bytes(&payload).await?)
    }

    async fn close(&mut self) -> Result<()> {
        Ok(self.stream.close().await?)
    }
}
