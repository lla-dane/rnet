use std::{
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use tokio::sync::Mutex;
use tracing::debug;

use crate::pubsub::LastSeenCache;

const LAST_SEEN_TTL: u64 = 120;
const MESSAGE_CACHE_CLEANUP: Duration = Duration::from_secs(100);

pub async fn cleanup_msg_cache(msg_cache: Arc<Mutex<LastSeenCache>>) {
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

pub fn seqno_to_unix_tx(seqno: &[u8]) -> Option<u64> {
    if seqno.len() != 8 {
        return None;
    }

    let mut arr = [0u8; 8];
    arr.copy_from_slice(seqno);

    Some(u64::from_le_bytes(arr))
}

#[cfg(test)]
mod test {
    use crate::cache::{get_seqno, seqno_to_unix_tx};

    #[test]
    fn test_seqno_roundtrip() {
        let timestamp = get_seqno();
        println!("{:?}", timestamp);
        println!("{}", timestamp.len());

        let ts = seqno_to_unix_tx(&timestamp).unwrap();
        println!("{}", ts);
    }
}
