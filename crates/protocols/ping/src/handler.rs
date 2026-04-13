use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::Result;
use async_trait::async_trait;
use identity::traits::{core::IProtocolHandler, muxer::IMuxedStream};
use tokio::sync::Mutex;
use tracing::{debug, error, warn};

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

    async fn ping(
        &self,
        stream: &mut Box<dyn IMuxedStream + Send + Sync + 'static>,
    ) -> Result<u128> {
        let payload = vec![0x01; PING_LENGTH];

        let rtt = match stream.is_initiator() {
            true => {
                let start = Instant::now();

                match stream.write(&payload).await {
                    Ok(()) => {}
                    Err(e) => {
                        error!("Error in sending ping: {}", e)
                    }
                }

                match stream.read().await {
                    Ok(_) => {}
                    Err(e) => {
                        error!("Error in receiving pong: {}", e)
                    }
                }

                start.elapsed().as_micros()
            }
            false => {
                match stream.read().await {
                    Ok(_) => {}
                    Err(e) => {
                        error!("Error in receiving ping: {}", e)
                    }
                }

                match stream.write(&payload).await {
                    Ok(()) => {}
                    Err(e) => {
                        error!("Error in sending pong: {}", e)
                    }
                }

                0
            }
        };

        Ok(rtt)
    }

    async fn infinite_ping(
        &self,
        stream: &mut Box<dyn IMuxedStream + Send + Sync + 'static>,
    ) -> Result<()> {
        let peer_id = stream.get_peer_id();
        match stream.is_initiator() {
            true => {
                for _ in 0..10 {
                    match self.ping(stream).await {
                        Err(e) => error!("Error in ping sequence: {}, {}", e, peer_id),
                        Ok(rtt) => {
                            debug!("Ping rtt exchange: RTT = [{}]", rtt)
                        }
                    }
                    tokio::time::sleep(Duration::from_millis(1000)).await;
                }
            }
            false => {
                for _ in 0..10 {
                    match self.ping(stream).await {
                        Err(e) => error!("Error in ping exchange: {}, {}", e, peer_id),
                        _ => {}
                    }
                    tokio::time::sleep(Duration::from_millis(1000)).await;
                }
            }
        };

        Ok(())
    }

    async fn handle_ping(
        &self,
        stream: &mut Box<dyn IMuxedStream + Send + Sync + 'static>,
        count: Option<u32>,
    ) -> Result<()> {
        let peer_id = stream.get_peer_id();

        match stream.is_initiator() {
            true => {
                let mut rtts = Vec::new();

                let count = count.unwrap();
                let count_bytes = count.to_be_bytes();

                stream.write(&count_bytes.to_vec()).await.unwrap();
                tokio::time::sleep(Duration::from_millis(50)).await;

                if count == 0 {
                    self.infinite_ping(stream).await.unwrap();
                    return Ok(());
                }

                for _ in 0..count {
                    match self.ping(stream).await {
                        Err(e) => error!("Error in ping sequence: {}, {}", e, peer_id),
                        Ok(rtt) => {
                            rtts.push(rtt);
                        }
                    }
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
                tokio::time::sleep(Duration::from_millis(50)).await;

                if ping_count == 0 {
                    self.infinite_ping(stream).await.unwrap();
                    return Ok(());
                }

                for _ in 0..ping_count {
                    match self.ping(stream).await {
                        Err(e) => error!("Error in ping sequence: {}, {}", e, peer_id),
                        Ok(_) => {}
                    }
                }

                debug!(
                    "Ping completed with {} RTTs from {}",
                    ping_count,
                    stream.get_peer_id(),
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
