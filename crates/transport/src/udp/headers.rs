use tracing::error;

pub enum UdpPacketFlag {
    Connect = 1,
    Disconnect = 2,
    General = 3,
}

pub fn serialize_udp_packet(payload: Vec<u8>, flag: UdpPacketFlag) -> Vec<u8> {
    let mut buf = Vec::new();

    match flag {
        UdpPacketFlag::Connect => {
            buf.push(UdpPacketFlag::Connect as u8);
            buf.extend(payload);
        }
        UdpPacketFlag::Disconnect => {
            buf.push(UdpPacketFlag::Disconnect as u8);
            buf.extend(payload);
        }
        UdpPacketFlag::General => {
            buf.push(UdpPacketFlag::General as u8);
            buf.extend(payload);
        }
    }

    buf
}

pub fn deserialize_udp_packet(buf: &[u8]) -> Option<(UdpPacketFlag, Vec<u8>)> {
    if buf.is_empty() {
        return None;
    }

    let flag = match buf[0] {
        x if x == UdpPacketFlag::Connect as u8 => UdpPacketFlag::Connect,
        x if x == UdpPacketFlag::Disconnect as u8 => UdpPacketFlag::Disconnect,
        x if x == UdpPacketFlag::General as u8 => UdpPacketFlag::General,
        _ => {
            error!("Unknow flag");
            return None;
        } // unknown flag
    };

    let payload = buf[1..].to_vec();
    Some((flag, payload))
}
