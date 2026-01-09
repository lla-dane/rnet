use std::{
    collections::VecDeque,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use rnet_proto::floodsub::Message;
use tokio::sync::{mpsc::Receiver, Mutex};
use tracing::debug;

use crate::{
    pubsub::{FloodSub, LastSeenCache},
    subscription::{process_floodsub_api_frame, SubAPIMpscFlag},
};

pub mod pubsub;
pub mod subscription;

const LAST_SEEN_TTL: u64 = 120;
const MESSAGE_CACHE_CLEANUP: Duration = Duration::from_secs(100);

pub async fn handle_floodsub_api(floodsub: Arc<FloodSub>, mut floodsub_mpsc_rx: Receiver<Vec<u8>>) {
    let mut notification = VecDeque::<Vec<u8>>::new();

    loop {
        tokio::select! {
            Some(event) = floodsub_mpsc_rx.recv() => {
                notification.push_back(event);
            }

            _ = async {}, if !notification.is_empty() => {
                let frame = notification.pop_front().unwrap();

                let (flag, (_, opt_vec_pld, opt_data)): (SubAPIMpscFlag, (Option<String>, Option<Vec<String>>, Option<Vec<u8>>)) = process_floodsub_api_frame(frame).unwrap();
                match flag {
                    SubAPIMpscFlag::DeadPeers => {
                        floodsub.handle_dead_peers(opt_vec_pld.unwrap())
                            .await.unwrap();
                    }
                    SubAPIMpscFlag::Unsubscribe => {
                        floodsub.unsubscribe(opt_vec_pld.unwrap()).await.unwrap();
                    }
                    SubAPIMpscFlag::Publish => {
                        let timestamp = get_seqno();

                        let pub_msg = Message {
                            from: Some(floodsub.local_peer_info.clone().peer_id.as_bytes().to_vec()),
                            data: opt_data,
                            seqno: Some(timestamp),
                            topic_ids: opt_vec_pld.unwrap(),
                        };

                        floodsub.publish(floodsub.local_peer_info.clone().peer_id, pub_msg).await.unwrap();
                    }
                }
            }
        }
    }
}

async fn cleanup_msg_cache(msg_cache: Arc<Mutex<LastSeenCache>>) {
    loop {
        {
            tokio::time::sleep(MESSAGE_CACHE_CLEANUP).await;
            let mut cache = msg_cache.lock().await;
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();
            cache.retain(|_, first_seen_ts| now.saturating_sub(*first_seen_ts) <= LAST_SEEN_TTL);
            debug!("Floodsub message cache cleanup");
        }
    }
}

pub fn get_seqno() -> Vec<u8> {
    let unix_ts: u64 = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    unix_ts.to_le_bytes().to_vec()
}

pub fn seqno_to_unix_tx(seqno: &Vec<u8>) -> Option<u64> {
    if seqno.len() != 8 {
        return None;
    }

    let mut arr = [0u8; 8];
    arr.copy_from_slice(seqno);

    Some(u64::from_le_bytes(arr))
}

#[cfg(test)]
mod test {
    use crate::{get_seqno, seqno_to_unix_tx};

    #[test]
    fn test_seqno_roundtrip() {
        let timestamp = get_seqno();
        println!("{:?}", timestamp);
        println!("{}", timestamp.len());

        let ts = seqno_to_unix_tx(&timestamp).unwrap();
        println!("{}", ts);
    }
}
