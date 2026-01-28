use rnet_traits::stream::IReadWriteClose;
use rnet_transport::RawConnection;

pub mod basic_host;
pub mod headers;
pub mod keys;
pub mod multiselect;

pub enum HandshakeReturn<T>
where
    T: IReadWriteClose,
{
    Raw(RawConnection<T>),
    Secure(),
    Muxed(),
}
