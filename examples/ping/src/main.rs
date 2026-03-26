#![allow(unused_variables, dead_code, unused_imports)]

use anyhow::Result;
use chacha20poly1305::{
    aead::{Aead, OsRng},
    AeadCore, ChaCha20Poly1305, ChaChaPoly1305, KeyInit,
};
use rnet_host::{
    basic_host::BasicHost,
    keys::{rsa::RsaKeyPair, Keys},
};
use rnet_mplex::{mplex::AsyncHandler, mplex_stream::MplexStream};
use rnet_multiaddr::Multiaddr;
use rnet_traits::host::IHostMpscTx;
use rnet_traits::stream::IMuxedStream;
use std::{env, sync::Arc};
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;
use x25519_dalek::PublicKey;

const PING_LENGTH: usize = 32;
const IPFS_PING: &str = "/ipfs/ping/1.0.0";

pub async fn handle_ping(stream: &mut MplexStream) -> Result<()> {
    let payload = vec![0x01; PING_LENGTH];

    if stream.is_initiator {
        println!("sending ping to {}", stream.remote_peer_info.peer_id);
        stream.write(&payload).await.unwrap();

        let response = match stream.read().await {
            Ok(res) => res,
            Err(e) => {
                warn!("Connection dropped: {}", e);
                return Ok(());
            }
        };

        if response == payload {
            println!("received pong from {}", stream.remote_peer_info.peer_id);
        }
    } else {
        let req = match stream.read().await {
            Ok(req) => req,
            Err(e) => {
                warn!("Connection dropped: {}", e);
                return Ok(());
            }
        };

        if req == payload {
            println!("received ping from {}", stream.remote_peer_info.peer_id);
            println!("sending pong to {}", stream.remote_peer_info.peer_id);

            stream.write(&payload).await.unwrap();
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new("trace"))
        .without_time()
        .with_target(false)
        .compact()
        .init();
    let args: Vec<String> = env::args().collect();

    // let csprng = OsRng {};
    // let alice_secret = x25519_dalek::EphemeralSecret::random_from_rng(csprng);
    // let bob_secret = x25519_dalek::EphemeralSecret::random_from_rng(csprng);

    // let alice_public = PublicKey::from(&alice_secret);
    // let bob_public = PublicKey::from(&bob_secret);

    // let hex = hex::encode(alice_public.as_bytes());
    // println!("{}", hex);

    // let alice_shared_secret = alice_secret.diffie_hellman(&bob_public);
    // let bob_shared_secret = bob_secret.diffie_hellman(&alice_public);

    // let alice_key = chacha20poly1305::Key::from_slice(alice_shared_secret.as_bytes());
    // let bob_key = chacha20poly1305::Key::from_slice(bob_shared_secret.as_bytes());

    // let alice_cipher = ChaCha20Poly1305::new(&alice_key);
    // let bob_cipher = ChaCha20Poly1305::new(&bob_key);

    // let nonce = ChaCha20Poly1305::generate_nonce(&mut OsRng);

    // let ciphertext = alice_cipher
    //     .encrypt(&nonce, b"top secret orgy".as_ref())
    //     .unwrap();
    // let plaintext = bob_cipher.decrypt(&nonce, ciphertext.as_ref()).unwrap();

    // let ciphertext = bob_cipher
    //     .encrypt(&nonce, b"top secret orgy haha".as_ref())
    //     .unwrap();
    // let plaintext = alice_cipher.decrypt(&nonce, ciphertext.as_ref()).unwrap();

    // assert_eq!(&plaintext, b"top secret orgy haha");

    let mut listen_addr = Multiaddr::new("ip4/127.0.0.1/tcp/0").unwrap();
    let (mut host, host_tx) = BasicHost::new(&mut listen_addr).await.unwrap();

    let mut mode = "server".to_string();
    let mut destination = "";
    if args.len() > 1 {
        mode = "client".to_string();
        destination = &args[1];
    }

    let peer_data = host.peer_data.lock().await.clone();

    let handler: AsyncHandler =
        Arc::new(|mut stream: MplexStream| Box::pin(async move { handle_ping(&mut stream).await }));
    host.set_stream_handler(IPFS_PING, handler).unwrap();

    let handle = tokio::spawn(async move {
        host.run().await.unwrap();
    });

    if mode == "server".to_string() {
        info!(
            "Run in new terminal: \ncargo run --bin ping --release {:?}",
            peer_data.peer_info.listen_addr.to_string()
        );
    } else {
        let multiaddr = Multiaddr::new(destination).unwrap();
        // host_tx.connect(&multiaddr).await?;

        host_tx
            .new_stream(&multiaddr.to_string(), vec![IPFS_PING.to_string()])
            .await
            .unwrap();
    }

    if let (Err(e),) = tokio::join!(handle) {
        return Err(e.into());
    }

    Ok(())
}
