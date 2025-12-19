use anyhow::Result;

#[derive(Debug, Clone)]
pub enum HostMpscTxFlag {
    NewStream,
    Connect,
    Disconnect,
}

impl HostMpscTxFlag {
    pub fn tag(&self) -> u8 {
        match self {
            &Self::NewStream => 1,
            &Self::Connect => 2,
            &Self::Disconnect => 3,
        }
    }

    pub fn from_tag(tag: u8) -> Option<Self> {
        match tag {
            1 => Some(Self::NewStream),
            2 => Some(Self::Connect),
            3 => Some(Self::Disconnect),
            _ => None,
        }
    }
}

pub fn build_host_frame(
    flag: HostMpscTxFlag,
    payload_string: String,
    opt_vec: Option<Vec<String>>,
) -> Vec<u8> {
    let mut buf = Vec::new();

    buf.push(flag.tag());

    let s_bytes = payload_string.as_bytes();
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

pub fn process_host_frame(
    payload: Vec<u8>,
) -> Result<(HostMpscTxFlag, (String, Option<Vec<String>>))> {
    let mut idx = 0;

    let tag = payload[idx];
    idx += 1;

    let flag = HostMpscTxFlag::from_tag(tag)
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

    Ok((flag, (s, opt_vec)))
}

#[cfg(test)]
mod tests {
    use rnet_multiaddr::Multiaddr;

    use super::*;

    #[test]
    fn test_roundtrip_with_vec() {
        let maddr = Multiaddr::new(
            "/ip4/127.0.0.1/tcp/38149/p2p/Fh7ayE1mzJH9zr5Th3qVhffxMftH5VkwhwcA3KxQzVR5",
        )
        .unwrap();

        let protocols = vec![String::from("/ipfs/ping/1.0.0")];

        let frame = build_host_frame(
            HostMpscTxFlag::Connect,
            maddr.to_string(),
            Some(protocols.clone()),
        );
        let (decoded_flag, (decoded_string, decoded_vec)) = process_host_frame(frame).unwrap();

        assert!(matches!(decoded_flag, HostMpscTxFlag::Connect));
        assert_eq!(decoded_string, maddr.to_string());
        assert_eq!(decoded_vec, Some(protocols));
    }
}
