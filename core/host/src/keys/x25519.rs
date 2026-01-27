use anyhow::Result;
use rand::rngs::OsRng;
use x25519_dalek::{EphemeralSecret, PublicKey, SharedSecret};

pub struct X25519KeyPair {
    pub private_key: EphemeralSecret,
    pub public_key: PublicKey,
}

impl X25519KeyPair {
    pub fn generate() -> Self {
        let pvt_key = EphemeralSecret::random_from_rng(&mut OsRng);
        let pub_key = PublicKey::from(&pvt_key);

        Self {
            private_key: pvt_key,
            public_key: pub_key,
        }
    }

    pub fn diffie_hellman(self, pubkey: &PublicKey) -> Result<SharedSecret> {
        Ok(self.private_key.diffie_hellman(pubkey))
    }

    pub fn get_public_key_hex(&self) -> String {
        hex::encode(self.public_key.as_bytes())
    }
}
