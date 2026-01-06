use anyhow::Result;
use tokio::sync::mpsc::{Receiver, Sender};

#[derive(Debug)]
pub struct SubscriptionAPI {
    floodsub_mpsc_tx: Sender<Vec<u8>>,
    topic_mpsc_tx: Sender<Vec<u8>>,
    topic_mpsc_rx: Receiver<Vec<u8>>,
}

impl SubscriptionAPI {
    pub fn new(
        floodsub_mpsc_tx: Sender<Vec<u8>>,
        topic_mpsc_tx: Sender<Vec<u8>>,
        topic_mpsc_rx: Receiver<Vec<u8>>,
    ) -> Self {
        return SubscriptionAPI {
            floodsub_mpsc_tx,
            topic_mpsc_tx,
            topic_mpsc_rx,
        };
    }

    pub async fn unsubscribe(&self) -> Result<()> {
        Ok(())
    }

    pub async fn send(&self, msg: Vec<u8>) -> Result<()> {
        Ok(())
    }

    pub async fn recv(&self) -> Result<Vec<u8>> {
        Ok(Vec::new())
    }
}

pub enum SubAPIMpscTxFlag {
    Unsubscribe,
    DeadPeers,
}

impl SubAPIMpscTxFlag {
    pub fn tag(&self) -> u8 {
        match self {
            &Self::Unsubscribe => 1,
            &Self::DeadPeers => 2,
        }
    }

    pub fn from_tag(tag: u8) -> Option<Self> {
        match tag {
            1 => Some(Self::Unsubscribe),
            2 => Some(Self::DeadPeers),
            _ => None,
        }
    }
}

pub fn build_floodsub_api_frame(
    flag: SubAPIMpscTxFlag,
    payload_string: Option<String>,
    opt_vec: Option<Vec<String>>,
) -> Vec<u8> {
    let mut buf = Vec::new();

    buf.push(flag.tag());

    let binding = payload_string.unwrap_or("none".to_string());
    let s_bytes = binding.as_bytes();

    buf.extend_from_slice(&(s_bytes.len() as u32).to_le_bytes());
    buf.extend_from_slice(s_bytes);

    match opt_vec {
        None => {
            buf.push(0);
        }
        Some(v) => {
            buf.push(1);
            buf.extend_from_slice(&(v.len() as u32).to_le_bytes());

            for item in v {
                let b = item.as_bytes();
                buf.extend_from_slice(&(b.len() as u32).to_le_bytes());
                buf.extend_from_slice(b);
            }
        }
    }

    buf
}

pub fn process_floodsub_api_frame(
    payload: Vec<u8>,
) -> Result<(SubAPIMpscTxFlag, (Option<String>, Option<Vec<String>>))> {
    let mut idx = 0;

    let tag = payload[idx];
    idx += 1;

    let flag = SubAPIMpscTxFlag::from_tag(tag)
        .ok_or_else(|| "invalid tag".to_string())
        .unwrap();

    // STRING-PAYLOAD
    let s_len = u32::from_le_bytes(payload[idx..idx + 4].try_into().unwrap()) as usize;
    idx += 4;

    let s = String::from_utf8(payload[idx..idx + s_len].to_vec()).unwrap();
    idx += s_len;

    // OPTIONAL VEC-PAYLOAD
    let has_vec = payload[idx];
    idx += 1;

    let s_payload = if s != "none".to_string() {
        Some(s)
    } else {
        None
    };

    let opt_vec = if has_vec == 0 {
        None
    } else {
        let count = u32::from_le_bytes(payload[idx..idx + 4].try_into().unwrap()) as usize;
        idx += 4;

        let mut v = Vec::with_capacity(count);
        for _ in 0..count {
            let len = u32::from_le_bytes(payload[idx..idx + 4].try_into().unwrap()) as usize;
            idx += 4;

            let item = String::from_utf8(payload[idx..idx + len].to_vec()).unwrap();
            idx += len;

            v.push(item);
        }

        Some(v)
    };

    Ok((flag, (s_payload, opt_vec)))
}
