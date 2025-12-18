#[derive(Debug, Clone)]
pub enum MuxedStreamFlag {
    NewStream,
    MessageResponse,
    MessageRequest,
    CloseStream,
    HandshakeReq,
    HandshakeRes,
}

impl MuxedStreamFlag {
    pub fn tag(&self) -> u8 {
        match self {
            Self::NewStream => 1,
            Self::MessageResponse => 2,
            Self::MessageRequest => 3,
            Self::CloseStream => 4,
            Self::HandshakeReq => 5,
            Self::HandshakeRes => 6,
        }
    }

    pub fn from_tag(tag: u8) -> Option<Self> {
        match tag {
            1 => Some(Self::NewStream),
            2 => Some(Self::MessageResponse),
            3 => Some(Self::MessageRequest),
            4 => Some(Self::CloseStream),
            5 => Some(Self::HandshakeReq),
            6 => Some(Self::HandshakeRes),
            _ => None,
        }
    }
}
