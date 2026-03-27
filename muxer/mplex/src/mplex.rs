use crate::headers::{build_frame, process_header};
use crate::{headers::MuxedStreamFlag, mplex_stream::MplexStream};

use anyhow::{Error, Result};
use async_trait::async_trait;
use rnet_peer::peer_info::PeerInfo;
use rnet_traits::conn::{IMuxedConn, IRawConnection, ISecuredConn};
use rnet_traits::host::IHostMpscTx;
use rnet_traits::stream::IMuxedStream;
use rnet_transport::RawConnection;

use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::result::Result::Ok;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::mpsc::{self, Receiver, Sender};

pub type AsyncHandler =
    Arc<dyn Fn(MplexStream) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> + Send + Sync>;

const INTERNAL: [u8; 16] = *b"internal-payload";

pub struct MplexConn<T>
where
    T: ISecuredConn,
{
    pub raw_conn: RawConnection<T>,
    pub remote_peer_info: PeerInfo,
    pub is_initiator: bool,
    pub streams: HashMap<u32, Sender<Vec<u8>>>,
    _stream_counter: u32,
    handlers: HashMap<String, AsyncHandler>,
    pub mpsc_tx: Sender<Vec<u8>>,
    pub mpsc_rx: Receiver<Vec<u8>>,
}

impl<T> MplexConn<T>
where
    T: ISecuredConn,
{
    pub fn new(
        raw_conn: RawConnection<T>,
        is_initiator: bool,
        remote_peer: PeerInfo,
        handlers: HashMap<String, AsyncHandler>,
        mpsc_tx: Sender<Vec<u8>>,
        mpsc_rx: Receiver<Vec<u8>>,
    ) -> MplexConn<T> {
        MplexConn {
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
}

#[async_trait]
impl<T> IMuxedConn for MplexConn<T>
where
    T: ISecuredConn + Send + Sync,
{
    async fn handle_incoming(&mut self, frames: Vec<u8>) -> Result<()> {
        // Process frames, to see the following:
        // - create a new muxed-stream as per the headers
        // - see which stream the payload belongs to, as per the headers

        // Extract the stream_id, header adn payload from the received frame
        let (stream_id, flag, _, payload_len) = process_header(&frames)?;
        let (_, remaining) = unsigned_varint::decode::u32(frames.as_slice()).unwrap();
        let (_, payload_after_len) = unsigned_varint::decode::usize(remaining).unwrap();
        let payload_extracted = payload_after_len[..payload_len].to_vec();

        match flag {
            MuxedStreamFlag::NewStream => {
                // Create a new stream
                let (muxed_stream_mpsc_tx, muxed_stream_mpsc_rx) = mpsc::channel::<Vec<u8>>(100);

                if !self.is_initiator {
                    let stream = MplexStream::new(
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
                    let stream = MplexStream::new(
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
                        b"".as_ref(),
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

    async fn conn_handler<W>(mut self, peer_id: &str, host_mpsc_tx: &Arc<W>) -> Result<()>
    where
        W: IHostMpscTx + Send + Sync,
    {
        let mut write_queue = VecDeque::<Vec<u8>>::new();

        loop {
            tokio::select! {
                Some(data) = self.mpsc_rx.recv() => {
                    if data.starts_with(&INTERNAL) {
                        self.handle_incoming(data[INTERNAL.len()..].to_vec()).await.unwrap();
                        continue;
                    }
                    write_queue.push_back(data);
                }

                frame = self.raw_conn.read() => {
                    match frame {
                        Ok(frames) => {
                            self.handle_incoming(frames).await.unwrap();
                        }
                        Err(_) => {break},
                    }
                }

                _ = async {}, if !write_queue.is_empty() => {
                    let data = write_queue.pop_front().unwrap();
                    if self.raw_conn.write(&data).await.is_err() {
                        break;
                    }
                }
            }
        }

        host_mpsc_tx.on_disconnect(peer_id).await.unwrap();

        Ok(())
    }

    async fn write(&self, msg: Vec<u8>) -> Result<()> {
        if let Err(e) = self.mpsc_tx.send(msg).await {
            return Err(Error::msg(format!("mpsc-receiver dropped: {}", e)));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::headers::{build_frame, process_header};

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
