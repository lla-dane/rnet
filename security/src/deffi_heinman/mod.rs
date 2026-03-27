use anyhow::Result;
use chacha20poly1305::{aead::OsRng, ChaCha20Poly1305, Key, KeyInit};
use rnet_traits::stream::IReadWriteClose;
use x25519_dalek::{EphemeralSecret, PublicKey};

pub struct DHTransport {}

impl DHTransport {
    pub async fn handshake<T>(&self, stream: &mut T, is_initiator: bool) -> Result<ChaCha20Poly1305>
    where
        T: IReadWriteClose,
    {
        // csprng - random generator
        // ephimeral secret-key generation
        // public-key generation
        // Exchange public key
        // generate shared-secret
        // generate/return cipher-key

        let csprng = OsRng {};

        let local_secret_key = EphemeralSecret::random_from_rng(csprng);
        let local_public_key = PublicKey::from(&local_secret_key);

        // Initiate local public-key exchange
        let remote_public_key = match is_initiator {
            true => {
                let pubkey_bytes = local_public_key.as_bytes().to_vec();
                stream.send_bytes(&pubkey_bytes).await?;

                let response = stream.recv_msg().await.unwrap();
                let remote_pubkey = PublicKey::from(<[u8; 32]>::try_from(response).unwrap());

                remote_pubkey
            }
            false => {
                let remote_pubkey_bytes = stream.recv_msg().await.unwrap();
                let remote_pubkey =
                    PublicKey::from(<[u8; 32]>::try_from(remote_pubkey_bytes).unwrap());

                let local_pubkey_bytes = local_public_key.as_bytes().to_vec();
                stream.send_bytes(&local_pubkey_bytes).await?;

                remote_pubkey
            }
        };

        // Generate the shared-secret
        let shared_secret = local_secret_key.diffie_hellman(&remote_public_key);
        let pre_cipher = Key::from_slice(shared_secret.as_bytes());
        let cipher_key = ChaCha20Poly1305::new(&pre_cipher);

        Ok(cipher_key)
    }
}
