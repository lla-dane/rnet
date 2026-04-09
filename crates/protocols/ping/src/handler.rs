use std::{sync::Arc, time::Instant};

use anyhow::Result;
use async_trait::async_trait;
use identity::traits::{core::IProtocolHandler, muxer::IMuxedStream};
use tokio::sync::Mutex;
use tracing::{debug, warn};

const PING_LENGTH: usize = 32;

pub struct PingEvent {
    pub is_initiator: bool,
    pub rtts: Vec<u128>,
    pub remote_peer_id: String,
}

pub struct Ping {
    pub count: Arc<Mutex<u32>>,
}

impl Ping {
    pub fn new(count: Option<u32>) -> Self {
        Ping {
            count: Arc::new(Mutex::new(count.unwrap_or(5))),
        }
    }

    async fn handle_ping(
        &self,
        stream: &mut Box<dyn IMuxedStream + Send + Sync + 'static>,
        count: Option<u32>,
    ) -> Result<()> {
        let payload = vec![0x01; PING_LENGTH];

        match stream.is_initiator() {
            true => {
                let count = count.unwrap();
                let count_bytes = count.to_be_bytes();

                stream.write(&count_bytes.to_vec()).await.unwrap();
                let mut rtts = Vec::new();

                for _ in 0..count {
                    let start = Instant::now();

                    stream.write(&payload).await.unwrap();
                    let response = match stream.read().await {
                        Ok(res) => res,
                        Err(e) => {
                            warn!("Connection dropped: {}", e);
                            break;
                        }
                    };

                    if response == payload {
                        let rtt = start.elapsed().as_micros();
                        rtts.push(rtt);
                        continue;
                    }

                    warn!("Invalid pong from {}", stream.get_peer_id());
                    break;
                }

                debug!(
                    "Ping completed with {} RTT samples from {}:\n{:?}",
                    rtts.len(),
                    stream.get_peer_id(),
                    rtts
                );
            }
            false => {
                let count_buf = stream.read().await.unwrap();
                if count_buf.len() < 4 {
                    warn!("Invalid ping-count received from {}", stream.get_peer_id());
                    return Ok(());
                }

                let ping_count =
                    u32::from_be_bytes([count_buf[0], count_buf[1], count_buf[2], count_buf[3]]);

                for _ in 0..ping_count {
                    let req = match stream.read().await {
                        Ok(req) => req,
                        Err(e) => {
                            warn!("Connection dropped : {}", e);
                            break;
                        }
                    };
                    stream.write(&req).await.unwrap();
                }

                debug!(
                    "Ping completed with {} RTTs from {}",
                    ping_count,
                    stream.get_peer_id()
                );
            }
        };

        Ok(())
    }
}

#[async_trait]
impl IProtocolHandler for Ping {
    async fn stream_handler(
        &self,
        mut stream: Box<dyn IMuxedStream + Send + Sync + 'static>,
    ) -> Result<()> {
        match stream.is_initiator() {
            true => {
                let ping_count: u32 = {
                    let ping_guard = self.count.lock().await;
                    *ping_guard
                };

                self.handle_ping(&mut stream, Some(ping_count))
                    .await
                    .unwrap();
            }
            false => self.handle_ping(&mut stream, None).await.unwrap(),
        };

        Ok(())
    }
}
