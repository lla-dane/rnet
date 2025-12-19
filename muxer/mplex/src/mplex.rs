use crate::{headers::MuxedStreamFlag, mplex_stream::MuxedStream};
use anyhow::{Error, Result};
use rnet_peer::peer_info::PeerInfo;
use rnet_transport::RawConnection;
use std::future::Future;
use std::pin::Pin;
use std::result::Result::Ok;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::mpsc::{self, Receiver, Sender};

pub type AsyncHandler =
    Arc<dyn Fn(MuxedStream) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> + Send + Sync>;

pub struct MuxedConn {
    pub raw_conn: RawConnection,
    pub remote_peer_info: PeerInfo,
    pub is_initiator: bool,
    pub streams: HashMap<u32, Sender<Vec<u8>>>,
    _stream_counter: u32,
    handlers: HashMap<String, AsyncHandler>,
    pub mpsc_tx: Sender<Vec<u8>>,
    pub mpsc_rx: Receiver<Vec<u8>>,
}

impl MuxedConn {
    pub fn new(
        raw_conn: RawConnection,
        is_initiator: bool,
        remote_peer: PeerInfo,
        handlers: HashMap<String, AsyncHandler>,
        mpsc_tx: Sender<Vec<u8>>,
        mpsc_rx: Receiver<Vec<u8>>,
    ) -> MuxedConn {
        MuxedConn {
            raw_conn,
            remote_peer_info: remote_peer,
            is_initiator,
            streams: HashMap::new(),
            _stream_counter: 0,
            handlers,
            mpsc_tx,
            mpsc_rx,
        }
    }

    pub async fn handle_incoming(&mut self, frames: Vec<u8>) -> Result<()> {
        // Process frames, to see the following:
        // - create a new muxed-stream as per the headers
        // - see which stream the payload belongs to, as per the headers

        // Extract the stream_id, header adn payload from the received frame
        let (stream_id, flag, _, payload_len) = process_header(&frames)?;
        let (_, remaining) = unsigned_varint::decode::u32(&frames.as_slice()).unwrap();
        let (_, payload_after_len) = unsigned_varint::decode::usize(remaining).unwrap();
        let payload_extracted = payload_after_len[..payload_len].to_vec();

        match flag {
            MuxedStreamFlag::NewStream => {
                // Create a new stream
                let (muxed_stream_mpsc_tx, muxed_stream_mpsc_rx) = mpsc::channel::<Vec<u8>>(100);

                if !self.is_initiator {
                    let stream = MuxedStream::new(
                        self.mpsc_tx.clone(),
                        muxed_stream_mpsc_rx,
                        stream_id,
                        self.is_initiator,
                        self.remote_peer_info.clone(),
                        self.handlers.clone(),
                    );
                    self.streams.insert(stream_id, muxed_stream_mpsc_tx.clone());

                    // SERVER HANDSHAKE PROCEDURE
                    tokio::spawn(async move {
                        stream.server_handshake().await.unwrap();
                    });
                } else {
                    self._stream_counter += 1;
                    let stream = MuxedStream::new(
                        self.mpsc_tx.clone(),
                        muxed_stream_mpsc_rx,
                        self._stream_counter,
                        self.is_initiator,
                        self.remote_peer_info.clone(),
                        self.handlers.clone(),
                    );
                    self.streams
                        .insert(self._stream_counter, muxed_stream_mpsc_tx.clone());

                    // INITIATOR-FRAME -> SERVER
                    let initiator_frame = build_frame(
                        self._stream_counter,
                        MuxedStreamFlag::NewStream,
                        &b"".to_vec(),
                    );
                    self.write(initiator_frame).await.unwrap();

                    // CLIENT HANDSHAKE PROCEDURE
                    tokio::spawn(async move {
                        stream.client_handshake(payload_extracted).await.unwrap();
                    });
                }
            }

            MuxedStreamFlag::MessageResponse | MuxedStreamFlag::MessageRequest => {
                let muxed_stream_mpsc_tx = self
                    .streams
                    .get_mut(&stream_id)
                    .expect("invalid stream-id")
                    .clone();
                muxed_stream_mpsc_tx.send(payload_extracted).await.unwrap();
            }

            MuxedStreamFlag::HandshakeRes => {
                let muxed_stream_mpsc_tx = self
                    .streams
                    .get_mut(&stream_id)
                    .expect("invalid stream-id")
                    .clone();
                muxed_stream_mpsc_tx.send(payload_extracted).await.unwrap();
            }

            MuxedStreamFlag::HandshakeReq => {
                let muxed_stream_mpsc_tx = self
                    .streams
                    .get_mut(&stream_id)
                    .expect("invalid stream-id")
                    .clone();
                muxed_stream_mpsc_tx.send(payload_extracted).await.unwrap();
            }

            MuxedStreamFlag::CloseStream => {
                self.streams.remove(&stream_id);
            }
        }

        Ok(())
    }

    pub async fn write(&self, msg: Vec<u8>) -> Result<()> {
        if let Err(e) = self.mpsc_tx.send(msg).await {
            return Err(Error::msg(format!("mpsc-receiver dropped: {}", e)));
        }

        Ok(())
    }
}

pub fn process_header(buf: &Vec<u8>) -> Result<(u32, MuxedStreamFlag, usize, usize)> {
    // decode varint header
    let (header_val, remaining) = unsigned_varint::decode::u32(buf.as_slice())
        .map_err(|_| anyhow::anyhow!("invalid varint header"))?;

    // How many bytes were consumed?
    let header_len = buf.len() - remaining.len();

    let tag = (header_val & 0b111) as u8;
    let stream_id = header_val >> 3;

    let (payload_len, _) =
        unsigned_varint::decode::usize(remaining).map_err(|_| anyhow::anyhow!("Invalid length"))?;

    let flag =
        MuxedStreamFlag::from_tag(tag).ok_or_else(|| anyhow::anyhow!("Invalid mplex flag"))?;

    Ok((stream_id, flag, header_len, payload_len))
}

pub fn build_frame(stream_id: u32, flag: MuxedStreamFlag, payload: &Vec<u8>) -> Vec<u8> {
    let mut buf = Vec::new();

    // ---- Encode header varint ----
    let header_val = (stream_id << 3) | (flag.tag() as u32);
    let mut header_tmp = [0u8; 5];
    let header_encoded = unsigned_varint::encode::u32(header_val, &mut header_tmp);
    buf.extend_from_slice(header_encoded);

    // ---- Encode length varint ----
    let mut len_tmp = [0u8; 10];
    let len_encoded = unsigned_varint::encode::usize(payload.len(), &mut len_tmp);
    buf.extend_from_slice(len_encoded);

    // ---- Payload ----
    buf.extend_from_slice(payload);

    buf
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_and_parse_frame() {
        let stream_id = 42u32;
        let flag = MuxedStreamFlag::MessageRequest;
        let payload = b"HelloWorld!".to_vec();

        let frame = build_frame(stream_id, flag.clone(), &payload);

        let (parsed_stream_id, parsed_flag, _, payload_len) = process_header(&frame).unwrap();

        assert_eq!(parsed_stream_id, stream_id);
        assert_eq!(parsed_flag.tag(), flag.tag(), "flag mismatch");
        assert_eq!(payload_len, payload.len(), "payload_len mismatch");

        let (_, remaining) = unsigned_varint::decode::u32(frame.as_slice()).unwrap();

        let (decoded_payload_len, payload_after_len) =
            unsigned_varint::decode::usize(remaining).unwrap();

        assert_eq!(decoded_payload_len, payload_len);
        let payload_extracted = payload_after_len[..payload_len].to_vec();

        assert_eq!(payload_extracted, payload, "payload mismatch");
    }
}
