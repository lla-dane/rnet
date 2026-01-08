use rnet_mplex::mplex::{build_frame, AsyncHandler, MuxedConn};
use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
};
use tokio::sync::{
    mpsc::{self, Receiver, Sender},
    Mutex,
};

use anyhow::{Error, Result};
use rnet_multiaddr::{Multiaddr, Protocol};
use rnet_peer::{peer_info::PeerInfo, PeerData};
use rnet_tcp::{TcpConn, TcpTransport};
use rnet_traits::transport::Transport;
use std::result::Result::Ok;
use tracing::{debug, info, warn};

use crate::{
    headers::{build_host_frame, process_host_frame, HostMpscTxFlag},
    keys::rsa::RsaKeyPair,
    multiselect::{multiselect::Multiselect, mutilselect_com::MultiselectComm},
};

const INTERNAL: [u8; 16] = *b"internal-payload";

#[derive(Debug)]
pub struct HostMpscTx {
    pub host_mpsc_tx: Sender<Vec<u8>>,
}

// #[derive(Debug)]
pub struct BasicHost {
    pub transport: TcpTransport,
    pub key_pair: RsaKeyPair,
    pub peer_data: Arc<Mutex<PeerData>>,
    pub connections: Arc<Mutex<HashMap<String, Sender<Vec<u8>>>>>,
    pub stream_handlers: HashMap<String, String>,
    pub multiselect: Multiselect,
    pub multiselect_comm: MultiselectComm,
    pub handlers: HashMap<String, AsyncHandler>,
    pub host_mpsc_tx: Arc<HostMpscTx>,
    pub host_mpsc_rx: Receiver<Vec<u8>>,
}

impl BasicHost {
    pub async fn new(listen_addr: &mut Multiaddr) -> Result<(Self, Arc<HostMpscTx>)> {
        let listener = TcpTransport::listen(&listen_addr).await.unwrap();
        let local_addr = listener.get_local_addr().unwrap();
        let parts: Vec<&str> = local_addr.split(':').collect();

        // ------ LISTEN-ADDR--------
        debug!("Generating RSA keypair");
        let keypair = RsaKeyPair::generate().unwrap();
        let peer_id = keypair.peer_id();

        listen_addr
            .replace_value_for_protocol("tcp", parts[1])
            .unwrap();

        listen_addr.push_proto(Protocol::P2P(peer_id.clone()));

        // ---------------------------

        info!("Host listening on: {}", listen_addr.to_string());
        let (host_mpsc_tx, host_mpsc_rx) = mpsc::channel::<Vec<u8>>(100);

        let host_tx = Arc::new(HostMpscTx { host_mpsc_tx });
        Ok((
            BasicHost {
                transport: listener,
                key_pair: keypair,
                peer_data: Arc::new(Mutex::new(PeerData {
                    peer_info: PeerInfo {
                        peer_id,
                        listen_addr: listen_addr.clone().to_string(),
                    },
                    peer_store: HashMap::new(),
                })),
                connections: Arc::new(Mutex::new(HashMap::new())),
                stream_handlers: HashMap::new(),
                multiselect: Multiselect {},
                multiselect_comm: MultiselectComm {},
                handlers: HashMap::new(),
                host_mpsc_tx: host_tx.clone(),
                host_mpsc_rx,
            },
            host_tx,
        ))
    }

    pub fn set_stream_handler(&mut self, protocol: &str, handler: AsyncHandler) -> Result<()> {
        self.handlers.insert(protocol.to_string(), handler);
        Ok(())
    }

    pub async fn run(&mut self) -> Result<()> {
        let mut notification = VecDeque::<Vec<u8>>::new();

        loop {
            tokio::select! {
                Ok((stream, _addr)) = self.transport.accept() => {
                    self.conn_handler(stream, false).await.unwrap();
                }

                Some(event) = self.host_mpsc_rx.recv() => {
                    notification.push_back(event);
                }

                _ = async {}, if !notification.is_empty() => {
                    let frame = notification.pop_front().unwrap();

                    // Process the data
                    let (flag, (str_pld, opt_vec_pld)): (HostMpscTxFlag, (String, Option<Vec<String>>)) = process_host_frame(frame).unwrap();
                    match flag {

                        HostMpscTxFlag::Connect => {
                            let maddr = Multiaddr::new(&str_pld).unwrap();
                            self.connect(&maddr).await.unwrap();
                        },

                        HostMpscTxFlag::NewStream => {
                            let peer_id = &str_pld;
                            let protocols = opt_vec_pld.unwrap();
                            self.new_stream(peer_id, protocols).await;
                        },

                        HostMpscTxFlag::Disconnect => {
                            let peer_id = &str_pld;
                            self.on_disconnect(peer_id).await.unwrap();
                        },
                    }


                }

            }
        }

        // TODO: ERROR HANDLING
    }

    pub async fn connect(&self, addr: &Multiaddr) -> Result<()> {
        let remote_peer_id = addr.value_for_protocol("p2p").unwrap();
        {
            let connections = self.connections.lock().await;
            if connections.contains_key(&remote_peer_id) {
                warn!("Peer already connected: {}", remote_peer_id);
                return Ok(());
            }
        }

        let stream = TcpTransport::dial(addr).await?;
        self.conn_handler(stream, true).await.unwrap();

        Ok(())
    }

    pub async fn new_stream(&self, peer_id: &str, protocols: Vec<String>) {
        let mut connections = self.connections.lock().await;
        let muxed_conn_mpsc_tx = connections
            .get_mut(peer_id)
            .ok_or_else(|| anyhow::anyhow!("No such peer"))
            .expect("Make a connection first")
            .clone();

        // INITIALIZE THE NEW-STREAM HEADER
        let mut new_stream_frame = build_frame(
            0,
            rnet_mplex::headers::MuxedStreamFlag::NewStream,
            &format!("{}", protocols[0]).as_bytes().to_vec(),
        );
        new_stream_frame.splice(0..0, INTERNAL);
        muxed_conn_mpsc_tx.send(new_stream_frame).await.unwrap();
    }

    pub async fn conn_handler(&self, stream: TcpConn, is_initiator: bool) -> Result<()> {
        let local_peer_info = {
            let data = self.peer_data.lock().await;
            data.peer_info.clone()
        };

        // security upgrade
        // Identify \/
        // stream_multiplexer upgrade
        // Active protocol negotiation

        // -------IDENTIFY-------
        let raw_conn = if !is_initiator {
            self.multiselect.handshake(&local_peer_info, stream).await?
        } else {
            self.multiselect_comm
                .handshake(&local_peer_info, stream)
                .await?
        };

        // ------TODO:-RUN-STREAM-MUXER-UPDATE-----

        let remote_peer_info = raw_conn.peer_info();
        info!("New peer connected: {}", remote_peer_info.peer_id);

        // -------MPEX-UPDATE-----
        let (muxed_conn_mpsc_tx, muxed_conn_mpsc_rx) = mpsc::channel::<Vec<u8>>(100);
        let muxed_conn = MuxedConn::new(
            raw_conn,
            is_initiator,
            remote_peer_info.clone(),
            self.handlers.clone(),
            muxed_conn_mpsc_tx.clone(),
            muxed_conn_mpsc_rx,
        );

        // ----UPDATE-PEER-STORE----
        {
            let mut peer_data = self.peer_data.lock().await;
            peer_data
                .peer_store
                .insert(remote_peer_info.peer_id.clone(), remote_peer_info.clone());

            let mut connections = self.connections.lock().await;
            connections.insert(remote_peer_info.peer_id.clone(), muxed_conn_mpsc_tx.clone());
        }

        //----SPAWN-MUXED-CONN-HANDLER----
        let host_mpsc_tx = self.host_mpsc_tx.clone();
        tokio::spawn(async move {
            host_mpsc_tx
                .handle_incoming(muxed_conn, remote_peer_info.peer_id.as_str())
                .await
                .unwrap();
        });

        Ok(())
    }

    async fn on_disconnect(&self, peer_id: &str) -> Result<()> {
        let mut peer_data = self.peer_data.lock().await;
        peer_data.peer_store.remove(peer_id);

        let mut connections = self.connections.lock().await;
        connections.remove(peer_id);
        warn!("Peer disconnected: {}", peer_id);

        Ok(())
    }
}

impl HostMpscTx {
    pub async fn connect(&self, maddr: &Multiaddr) -> Result<()> {
        let frame = build_host_frame(HostMpscTxFlag::Connect, maddr.to_string(), None);
        self.write(frame).await.unwrap();
        Ok(())
    }

    pub async fn new_stream(&self, peer_id: &str, protocols: Vec<String>) -> Result<()> {
        let frame = build_host_frame(
            HostMpscTxFlag::NewStream,
            peer_id.to_string(),
            Some(protocols),
        );
        self.write(frame).await.unwrap();
        Ok(())
    }

    pub async fn on_disconnect(&self, peer_id: &str) -> Result<()> {
        let frame = build_host_frame(HostMpscTxFlag::Disconnect, peer_id.to_string(), None);
        self.write(frame).await.unwrap();
        Ok(())
    }

    pub async fn write(&self, notification: Vec<u8>) -> Result<()> {
        if let Err(e) = self.host_mpsc_tx.send(notification).await {
            return Err(Error::msg(format!("mpsc-receiver dropped: {}", e)));
        }

        Ok(())
    }

    pub async fn handle_incoming(&self, mut muxed_conn: MuxedConn, peer_id: &str) -> Result<()> {
        let mut write_queue = VecDeque::<Vec<u8>>::new();

        loop {
            tokio::select! {
                // TODO: VULNERABILITY HERE
                Some(data) = muxed_conn.mpsc_rx.recv() => {

                    if data.starts_with(&INTERNAL) {
                        muxed_conn.handle_incoming(data[INTERNAL.len()..].to_vec()).await.unwrap();
                        continue;
                    }
                    write_queue.push_back(data);
                }

                frame = muxed_conn.raw_conn.read() => {
                    match frame {
                        Ok(frames) => {
                            muxed_conn.handle_incoming(frames).await.unwrap();
                        }
                        Err(_) => {break},
                    }
                }

                _ = async {}, if !write_queue.is_empty() => {
                    let data = write_queue.pop_front().unwrap();
                    if let Err(_) = muxed_conn.raw_conn.write(&data).await {
                        break;
                    }
                }
            }
        }

        self.on_disconnect(peer_id).await.unwrap();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_headers() {
        let maddr = Multiaddr::new("ip4/127.0.0.1/tcp/0")
            .unwrap()
            .to_string()
            .into_bytes();

        let string = String::from_utf8(maddr).unwrap();
        println!("{}", string);
    }
}
