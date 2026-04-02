use anyhow::Result;

pub mod conn;
pub mod deffi_hellman;
pub mod transport;
pub mod upgrader;

pub trait ISecureCipher {
    fn idecrypt(&self, nonce: &[u8], ciphertext: &[u8]) -> Result<Vec<u8>>;
    fn iencrypt(&self, msg: &[u8]) -> Result<(Vec<u8>, Vec<u8>)>;
}
