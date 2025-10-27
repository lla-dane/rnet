use ::rsa::pkcs1v15::Signature;
use anyhow::Result;

pub mod rsa;

pub trait Keys {
    fn public_key(&self) -> String;
    fn sign(&self, msg: &[u8]) -> Result<Signature>;
    fn verify(&self, msg: &[u8], sig: &Signature) -> bool;
}
