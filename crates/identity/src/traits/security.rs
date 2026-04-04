use anyhow::Result;
use async_trait::async_trait;

#[async_trait]
pub trait ISecuredConn {
    async fn read(&mut self) -> Result<Vec<u8>>;

    #[allow(clippy::ptr_arg)]
    async fn write(&mut self, msg: &Vec<u8>) -> Result<()>;
    async fn close(&mut self) -> Result<()>;
}
pub trait ISecureCipher {
    fn idecrypt(&self, nonce: &[u8], ciphertext: &[u8]) -> Result<Vec<u8>>;
    fn iencrypt(&self, msg: &[u8]) -> Result<(Vec<u8>, Vec<u8>)>;
}
