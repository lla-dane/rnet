use anyhow::{Ok, Result};

#[derive(Debug)]
pub enum MuxedStreamFlag {
    NewStreamInitiator,
    NewStreamReceiver,
    MessageInitiator,
    MessageReceiver,
    CloseInitiator,
    CloseReceiver,
}

impl MuxedStreamFlag {
    fn tag(&self) -> u8 {
        match self {
            Self::NewStreamInitiator => 0,
            Self::NewStreamReceiver => 1,
            Self::MessageReceiver => 2,
            Self::MessageInitiator => 3,
            Self::CloseReceiver => 4,
            Self::CloseInitiator => 5,
        }
    }

    fn from_tag(tag: u8) -> Option<Self> {
        match tag {
            0 => Some(Self::NewStreamInitiator),
            1 => Some(Self::NewStreamReceiver),
            2 => Some(Self::MessageReceiver),
            3 => Some(Self::MessageInitiator),
            4 => Some(Self::CloseReceiver),
            5 => Some(Self::CloseInitiator),
            _ => None,
        }
    }
}

pub fn create_header(stream_id: u32, flag: MuxedStreamFlag) -> Vec<u8> {
    let header_val = (stream_id << 3) | (flag.tag() as u32);

    let mut buf = [0u8; 5];
    let encoded = unsigned_varint::encode::u32(header_val, &mut buf);

    encoded.to_vec()
}

pub fn process_header(payload: &Vec<u8>) -> Result<()> {
    // decode varint header
    let (header_val, consumed) = unsigned_varint::decode::u32(&payload.as_slice())


    Ok(())
}
