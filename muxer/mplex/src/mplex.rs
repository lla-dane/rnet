use anyhow::{Error, Ok, Result};
use rnet_peer::peer_info::PeerInfo;
use std::collections::HashMap;
use tokio::sync::mpsc::{self, Receiver, Sender};
use tracing::{debug, info};

use crate::{headers::MuxedStreamFlag, mplex_stream::MuxedStream};

#[derive(Debug)]
pub struct MuxedConn {
    pub remote_peer_info: PeerInfo,
    pub is_initiator: bool,
    pub streams: HashMap<u32, Sender<Vec<u8>>>,
    _stream_counter: u32,
    handlers: HashMap<String, String>,
    pub conn_sender: Sender<Vec<u8>>,
    pub conn_receiver: Receiver<Vec<u8>>,
}

// There needs to be a protocol -> Handler mapping for streams

impl MuxedConn {
    pub fn new(
        is_initiator: bool,
        remote_peer: PeerInfo,
        handlers: HashMap<String, String>,
        conn_sender: Sender<Vec<u8>>,
        conn_receiver: Receiver<Vec<u8>>,
    ) -> MuxedConn {
        MuxedConn {
            remote_peer_info: remote_peer,
            is_initiator,
            streams: HashMap::new(),
            _stream_counter: 0,
            handlers,
            conn_sender,
            conn_receiver,
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
        let protocols: Vec<String> = self.handlers.keys().cloned().collect();

        match flag {
            MuxedStreamFlag::NewStream => {
                // Create a new stream
                let (tx, rx) = mpsc::channel::<Vec<u8>>(100);

                if !self.is_initiator {
                    let mut stream = MuxedStream::new(
                        self.conn_sender.clone(),
                        rx,
                        stream_id,
                        self.is_initiator,
                        self.remote_peer_info.clone(),
                        protocols,
                    );
                    self.streams.insert(stream_id, tx.clone());
                    info!("[SERVER] MUXED-CONN: NEW STREAM {}", stream_id);

                    // SERVER HANDSHAKE PROCEDURE
                    tokio::spawn(async move {
                        stream.server_handshake().await.unwrap();
                    });
                } else {
                    self._stream_counter += 1;
                    let mut stream = MuxedStream::new(
                        self.conn_sender.clone(),
                        rx,
                        self._stream_counter,
                        self.is_initiator,
                        self.remote_peer_info.clone(),
                        protocols,
                    );
                    self.streams.insert(self._stream_counter, tx.clone());

                    let initiator_frame = build_frame(
                        self._stream_counter,
                        MuxedStreamFlag::NewStream,
                        &b"Initiate".to_vec(),
                    );
                    self.conn_sender.send(initiator_frame).await.unwrap();
                    info!("[CLIENT] MUXED-CONN: NEW STREAM {}", self._stream_counter);

                    // CLIENT HANDSHAKE PROCEDURE
                    tokio::spawn(async move {
                        stream.client_handshake().await.unwrap();
                    });
                }

                // Carry out protocol negotiation
            }

            MuxedStreamFlag::MessageResponse | MuxedStreamFlag::MessageRequest => {
                let muxed_stream_tx = self
                    .streams
                    .get_mut(&stream_id)
                    .expect("invalid stream-id")
                    .clone();
                muxed_stream_tx.send(payload_extracted).await.unwrap();
            }

            MuxedStreamFlag::HandshakeRes => {
                debug!("[CLIENT]: HANDSHAKE RESPONSE CAME");
                let muxed_stream_tx = self
                    .streams
                    .get_mut(&stream_id)
                    .expect("invalid stream-id")
                    .clone();
                muxed_stream_tx.send(payload_extracted).await.unwrap();
            }

            MuxedStreamFlag::HandshakeReq => {
                debug!("[SERVER]: HANDSHAKE REQUEST CAME");
                let muxed_stream_tx = self
                    .streams
                    .get_mut(&stream_id)
                    .expect("invalid stream-id")
                    .clone();
                muxed_stream_tx.send(payload_extracted).await.unwrap();
            }

            MuxedStreamFlag::CloseStream => {
                self.streams.remove(&stream_id);
            }
        }

        Ok(())
    }

    pub async fn write(&self, msg: Vec<u8>) -> Result<()> {
        if let Err(_) = self.conn_sender.send(msg).await {
            return Err(Error::msg("Receiver dropped"));
        }

        Ok(())
    }

    pub async fn conn_handler(&mut self) {
        loop {
            tokio::select! {
                Some(frames) = self.conn_receiver.recv() => {
                    self.handle_incoming(frames).await.unwrap();
                }
            }
        }
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
