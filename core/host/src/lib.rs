use rnet_transport::RawConnection;

pub mod basic_host;
pub mod headers;
pub mod keys;
pub mod multiselect;

pub enum HandshakeReturn {
    Raw(RawConnection),
    Secure(),
    Muxed(),
}
