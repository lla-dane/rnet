use std::time::{SystemTime, UNIX_EPOCH};

pub mod pubsub;
pub mod subscription;

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
