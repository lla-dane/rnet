use std::time::Instant;

use anyhow::Result;
use identity::traits::muxer::IMuxedStream;
use tracing::{debug, warn};

const PING_LENGTH: usize = 32;

pub struct PingEvent {
    pub is_initiator: bool,
    pub rtts: Vec<u128>,
    pub remote_peer_id: String,
}

pub struct Ping {}

impl Ping {
    async fn _handle_ping<T>(&self, stream: &mut T) -> Option<u128>
    where
        T: IMuxedStream,
    {
        let payload = vec![0x01; PING_LENGTH];

        match stream.is_initiator() {
            true => {
                let start = Instant::now();
                debug!("sending ping to {}", stream.get_peer_id());

                stream.write(&payload).await.unwrap();
                let response = match stream.read().await {
                    Ok(res) => res,
                    Err(e) => {
                        warn!("Connection dropped: {}", e);
                        return None;
                    }
                };

                if response == payload {
                    debug!("received pong from {}", stream.get_peer_id());
                    let end = Instant::now();

                    let rtt = end.duration_since(start).as_micros();
                    return Some(rtt);
                }
                None
            }
            false => {
                let req = match stream.read().await {
                    Ok(req) => req,
                    Err(e) => {
                        warn!("Connection dropped: {}", e);
                        return None;
                    }
                };

                if req == payload {
                    debug!("received ping from {}", stream.get_peer_id());
                    debug!("sending pong to {}", stream.get_peer_id());

                    stream.write(&payload).await.unwrap();
                }

                None
            }
        }
    }

    pub async fn handle_ping<T>(&self, stream: &mut T, count: u8) -> Result<PingEvent>
    where
        T: IMuxedStream,
    {
        let remote_peer_id = stream.get_peer_id();
        let mut rtts: Vec<u128> = Vec::new();

        for _ in 0..count {
            if let Some(rtt) = self._handle_ping(stream).await {
                rtts.push(rtt);
            }
        }

        Ok(PingEvent {
            is_initiator: stream.is_initiator(),
            rtts,
            remote_peer_id,
        })
    }
}
