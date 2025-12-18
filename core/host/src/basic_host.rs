use rnet_mplex::mplex::{build_frame, MuxedConn};
use rnet_transport::RawConnection;
use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
};
use tokio::sync::{
    mpsc::{self, Receiver, Sender},
    Mutex,
};

use anyhow::Result;
use rnet_multiaddr::{Multiaddr, Protocol};
use rnet_peer::{peer_info::PeerInfo, PeerData};
use rnet_tcp::{TcpConn, TcpTransport};
use rnet_traits::transport::Transport;
use std::result::Result::Ok;
use tracing::{debug, info, warn};

use crate::{
    keys::rsa::RsaKeyPair,
    multiselect::{multiselect::Multiselect, mutilselect_com::MultiselectComm},
};

#[derive(Debug)]
pub struct BasicHost {
    pub transport: TcpTransport,
    pub key_pair: RsaKeyPair,
    pub peer_data: Arc<Mutex<PeerData>>,
    pub connections: Arc<Mutex<HashMap<String, Sender<Vec<u8>>>>>,
    pub stream_handlers: HashMap<String, String>,
    pub multiselect: Multiselect,
    pub multiselect_comm: MultiselectComm,
    pub handlers: HashMap<String, String>,
}

impl BasicHost {
    pub async fn new(listen_addr: &mut Multiaddr) -> Result<Arc<Self>> {
        let listener = TcpTransport::listen(&listen_addr).await.unwrap();
        let local_addr = listener.get_local_addr().unwrap();
        let parts: Vec<&str> = local_addr.split(':').collect();

        // ------ LISTEN-ADDR--------
        debug!("Generating RSA keypair");
        let keypair = RsaKeyPair::generate()?;
        let peer_id = keypair.peer_id();

        listen_addr
            .replace_value_for_protocol("tcp", parts[1])
            .unwrap();

        listen_addr.push_proto(Protocol::P2P(peer_id.clone()));

        // ---------------------------

        info!("Host listening on: {:?}", listen_addr.to_string());

        Ok(Arc::new(BasicHost {
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
        }))
    }

    pub async fn run(self: &Arc<Self>) -> Result<()> {
        loop {
            let (stream, _addr) = self.transport.accept().await?;

            let host = self.clone();
            tokio::spawn(async move {
                host.conn_handler(stream, false).await.unwrap();
            });
        }
    }

    pub async fn connect(self: &Arc<Self>, addr: &Multiaddr) -> Result<()> {
        let stream = TcpTransport::dial(addr).await?;
        let _peer_info = self.conn_handler(stream, true).await.unwrap();

        Ok(())
    }

    pub async fn new_stream(self: &Arc<Self>, peer_id: String, _protocols: Vec<String>) {

        let mut connections = self.connections.lock().await;
        let muxed_conn = connections
            .get_mut(&peer_id)
            .ok_or_else(|| anyhow::anyhow!("No such peer"))
            .expect("Make a connection first")
            .clone();

        // Send the new stream header to the client_receiver
        let new_stream_frame = build_frame(
            0,
            rnet_mplex::headers::MuxedStreamFlag::NewStream,
            &b"Initiate".to_vec(),
        );
        muxed_conn.send(new_stream_frame).await.unwrap();
    }

    pub async fn conn_handler(
        self: &Arc<Self>,
        stream: TcpConn,
        is_initiator: bool,
    ) -> Result<PeerInfo> {
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

        let peer_info = raw_conn.peer_info();
        info!("New peer connected: {}", peer_info.peer_id);

        // ------MPSC-CHANNELS-----
        let (tx1, rx1) = mpsc::channel::<Vec<u8>>(100);
        let (tx2, rx2) = mpsc::channel::<Vec<u8>>(100);

        // -------MPEX-UPDATE-----
        let mut muxed_conn = MuxedConn::new(
            is_initiator,
            peer_info.clone(),
            self.handlers.clone(),
            tx2.clone(),
            rx1,
        );

        info!("CONNECTION updated to MPLEX");

        let host = self.clone();
        let peer = peer_info.clone();

        {
            let mut peer_data = self.peer_data.lock().await;
            peer_data
                .peer_store
                .insert(peer_info.peer_id.clone(), peer_info.clone());

            let mut connections = self.connections.lock().await;
            connections.insert(peer_info.peer_id.clone(), tx1.clone());
        }

        // tokio spawn the handle_incoming of muxed connection
        tokio::spawn(async move {
            muxed_conn.conn_handler().await;
        });

        tokio::spawn(async move {
            host.handle_incoming(raw_conn, peer.peer_id.as_str(), rx2, tx1)
                .await
                .unwrap();
        });

        Ok(peer_info)
    }

    pub async fn handle_incoming(
        self: &Arc<Self>,
        mut raw_conn: RawConnection,
        peer_id: &str,
        mut receiver: Receiver<Vec<u8>>,
        sender: Sender<Vec<u8>>,
    ) -> Result<()> {
        let mut write_queue = VecDeque::<Vec<u8>>::new();

        loop {
            tokio::select! {

                Some(data) = receiver.recv() => {
                    write_queue.push_back(data);
                }

                frame = raw_conn.read() => {
                    match frame {
                        Ok(frames) => {
                            sender.send(frames).await?;
                        }
                        Err(e) => {
                            warn!("READ FAILED, DISCONNECTION: {:?}", e);
                            break;
                        }
                    }
                }

                _ = async {}, if !write_queue.is_empty() => {
                    let data = write_queue.pop_front().unwrap();
                    if let Err(e) = raw_conn.write(&data).await {
                        warn!("WRITE FAILED, DISCONNECTION: {:?}", e);
                        break;
                    }
                }
            }
        }

        self.on_disconnect(&peer_id).await?;
        Ok(())
    }

    async fn on_disconnect(self: &Arc<Self>, peer_id: &str) -> Result<()> {

        let mut peer_data = self.peer_data.lock().await;
        peer_data.peer_store.remove(peer_id);

        let mut connections = self.connections.lock().await;
        connections.remove(peer_id);
        debug!("Peer disconnected: {}", peer_id);

        Ok(())
    }
}
