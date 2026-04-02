use crate::conn::ISecureCipher;
use anyhow::Result;
use chacha20poly1305::{aead::Aead, AeadCore, Nonce};
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
                PublicKey::from(<[u8; 32]>::try_from(response).unwrap())
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
        let cipher_key = ChaCha20Poly1305::new(pre_cipher);

        Ok(cipher_key)
    }
}

impl ISecureCipher for ChaCha20Poly1305 {
    fn idecrypt(&self, nonce_bytes: &[u8], ciphertext: &[u8]) -> Result<Vec<u8>> {
        let nonce = Nonce::from_slice(nonce_bytes);

        let plaintext = self.decrypt(nonce, ciphertext).unwrap();
        Ok(plaintext)
    }

    fn iencrypt(&self, plaintext: &[u8]) -> Result<(Vec<u8>, Vec<u8>)> {
        let nonce = ChaCha20Poly1305::generate_nonce(&mut OsRng);
        let ciphertext = self.encrypt(&nonce, plaintext).unwrap();

        Ok((ciphertext, nonce.as_slice().to_vec()))
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
