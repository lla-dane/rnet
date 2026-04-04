use async_trait::async_trait;
use keys::rsa::RsaKeyPair;

use muxer::{
    mplex::{
        conn::AsyncHandler,
        headers::{build_frame, MuxedStreamFlag},
    },
    transport::MuxerTransport,
};
use security::{conn::SecureConn, transport::SecureTransport};
use transport::{raw_conn::RawConnection, tcp::transport::TcpTransport};
use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
};
use tokio::sync::{
    mpsc::{self, Receiver, Sender},
    Mutex,
};

use anyhow::{Error, Result};
use multiaddr::{Multiaddr, Protocol};
use peer::{peer_info::PeerInfo, PeerData};
use traits::{
    core::{IHostMpscTx, IMultistream, IReadWriteClose},
    muxer::IMuxedConn,
    transport::ITransport,
};
use std::result::Result::Ok;
use tracing::{debug, info, warn};

use crate::{
    headers::{build_host_frame, process_host_frame, HostMpscTxFlag},
    multistream::multiselect::Multiselect,
    protocol::{InnerProtocol, InnerProtocolOpt},
};

const INTERNAL: [u8; 16] = *b"internal-payload";

// TODO: After sometime:
// Only one endpoint to connect with the whole rnet node: HostMpscTx
// Set protocols which are going to be active in host-initiation
// All the internal function calls will be event driven from HostMpscTx,
// and received from global_event_rx
// Integrate Swarm with BasicHost
// Fix the dependency tree

// Happy exams !!

#[derive(Debug)]
pub struct HostMpscTx {
    pub key_pair: RsaKeyPair,
    pub host_mpsc_tx: Sender<Vec<u8>>,
}

pub struct BasicHost {
    pub transport: TcpTransport,
    pub key_pair: RsaKeyPair,
    pub peerstore: Arc<Mutex<PeerData>>,
    pub connections: Arc<Mutex<HashMap<String, Sender<Vec<u8>>>>>,
    pub stream_handlers: HashMap<String, String>,
    pub secure_transport: SecureTransport,
    pub muxer_transport: MuxerTransport,
    pub multiselect: Multiselect,
    pub handlers: HashMap<String, AsyncHandler>,
    pub host_mpsc_tx: Arc<HostMpscTx>,
    pub host_mpsc_rx: Receiver<Vec<u8>>,
    pub global_event_tx: Sender<Vec<u8>>,
    pub protocols: Vec<InnerProtocol>,
}
/// new
/// run
/// connect
/// new_stream
/// get_peer_id
/// get_addrs
/// get_pubkey
/// get_privkey
/// get_connected_peers
/// set_stream_handler
/// remove_stream_handler
/// disconnect
/// get_live_peers
/// is_peer_connected
impl BasicHost {
    pub async fn new(
        listen_addr: &mut Multiaddr,
        protocol_opt: Vec<InnerProtocolOpt>,
    ) -> Result<(Self, Arc<HostMpscTx>, Receiver<Vec<u8>>)> {
        let listener = TcpTransport::listen(listen_addr).await.unwrap();
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
        let (global_event_tx, global_event_rx) = mpsc::channel::<Vec<u8>>(100);

        let host_tx = Arc::new(HostMpscTx {
            host_mpsc_tx,
            key_pair: keypair.clone(),
        });
        Ok((
            BasicHost {
                transport: listener,
                key_pair: keypair,
                peerstore: Arc::new(Mutex::new(PeerData {
                    peer_info: PeerInfo {
                        peer_id,
                        listen_addr: listen_addr.clone().to_string(),
                    },
                    peer_store: HashMap::new(),
                })),
                connections: Arc::new(Mutex::new(HashMap::new())),
                stream_handlers: HashMap::new(),
                secure_transport: SecureTransport::new(),
                muxer_transport: MuxerTransport::new(),
                multiselect: Multiselect {},
                handlers: HashMap::new(),
                host_mpsc_tx: host_tx.clone(),
                host_mpsc_rx,
                global_event_tx,
                protocols: vec![],
            },
            host_tx,
            global_event_rx,
        ))
    }

    async fn execute_protocols(&self, protocol_opt: Vec<InnerProtocolOpt>) -> Result<()> {
        for opt in protocol_opt {
            match opt {
                InnerProtocolOpt::Floodsub => {}
                InnerProtocolOpt::Ping => {}
            }
        }

        Ok(())
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
                            let maadr = Multiaddr::new(&str_pld).unwrap();
                            let protocols = opt_vec_pld.unwrap();
                            self.new_stream(&maadr, protocols).await;
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

        // TODO: Set up a unified transport-opt here
        let stream = TcpTransport::dial(addr).await?;
        self.conn_handler(stream, true).await.unwrap();

        Ok(())
    }

    pub async fn new_stream(&self, maddr: &Multiaddr, protocols: Vec<String>) {
        let peer_id = maddr.value_for_protocol("p2p").unwrap();
        if !self.is_peer_connected(&peer_id).await {
            info!("Making a connection first: {}", peer_id);
            self.connect(maddr).await.unwrap();
        }
        let mut connections = self.connections.lock().await;

        let muxed_conn_mpsc_tx = connections
            .get_mut(&peer_id)
            .ok_or_else(|| anyhow::anyhow!("No such peer"))
            .expect("Make a connection first")
            .clone();

        // INITIALIZE THE NEW-STREAM HEADER
        let mut new_stream_frame =
            build_frame(0, MuxedStreamFlag::NewStream, protocols[0].as_bytes());
        new_stream_frame.splice(0..0, INTERNAL);
        muxed_conn_mpsc_tx.send(new_stream_frame).await.unwrap();
    }

    pub async fn is_peer_connected(&self, peer_id: &str) -> bool {
        let is_connected = {
            let connections = self.connections.lock().await;
            connections.contains_key(peer_id)
        };

        is_connected
    }

    pub async fn conn_handler<T>(&self, stream: T, is_initiator: bool) -> Result<()>
    where
        T: IReadWriteClose + Send + Sync + 'static,
    {
        let local_peer_info = {
            let data = self.peerstore.lock().await;
            data.peer_info.clone()
        };

        // security upgrade
        // Identify \/
        // stream_multiplexer upgrade
        // Active protocol negotiation

        // -------SECURITY-------
        let secured_conn = self
            .secure_transport
            .secure_conn(stream, is_initiator)
            .await
            .unwrap();

        // -------IDENTIFY-------
        let raw_conn: RawConnection<SecureConn<T>> = self
            .multiselect
            .handshake(&local_peer_info, secured_conn, is_initiator)
            .await
            .unwrap();

        // ------TODO:-RUN-STREAM-MUXER-UPDATE-----

        let remote_peer = raw_conn.peer_info();
        info!("New peer connected: {}", remote_peer.peer_id);

        // -------MUXER-UPDATE-----
        let (mut muxed_conn, muxed_conn_mpsc_tx) = self
            .muxer_transport
            .mux_conn(
                raw_conn,
                is_initiator,
                remote_peer.clone(),
                self.handlers.clone(),
                self.global_event_tx.clone(),
            )
            .await
            .unwrap();

        // ----UPDATE-PEER-STORE----
        {
            let mut peerstore = self.peerstore.lock().await;
            peerstore
                .peer_store
                .insert(remote_peer.peer_id.clone(), remote_peer.clone());

            let mut connections = self.connections.lock().await;
            connections.insert(remote_peer.peer_id.clone(), muxed_conn_mpsc_tx.clone());
        }

        //----SPAWN-MUXED-CONN-HANDLER----
        let host_mpsc_tx = self.host_mpsc_tx.clone();

        tokio::spawn(async move {
            muxed_conn
                .conn_handler(remote_peer.peer_id.as_str(), host_mpsc_tx)
                .await
                .unwrap();
        });

        Ok(())
    }

    async fn on_disconnect(&self, peer_id: &str) -> Result<()> {
        let mut peerstore = self.peerstore.lock().await;
        peerstore.peer_store.remove(peer_id);

        let mut connections = self.connections.lock().await;
        connections.remove(peer_id);
        warn!("Peer disconnected: {}", peer_id);

        Ok(())
    }
}

#[async_trait]
impl IHostMpscTx for HostMpscTx {
    async fn connect(&self, maddr: &Multiaddr) -> Result<()> {
        let frame = build_host_frame(HostMpscTxFlag::Connect, maddr.to_string(), None);
        self.write(frame).await.unwrap();
        Ok(())
    }

    async fn new_stream(&self, maddr: &str, protocols: Vec<String>) -> Result<()> {
        let frame = build_host_frame(
            HostMpscTxFlag::NewStream,
            maddr.to_string(),
            Some(protocols),
        );
        self.write(frame).await.unwrap();
        Ok(())
    }

    async fn on_disconnect(&self, peer_id: &str) -> Result<()> {
        let frame = build_host_frame(HostMpscTxFlag::Disconnect, peer_id.to_string(), None);
        self.write(frame).await.unwrap();
        Ok(())
    }

    async fn write(&self, notification: Vec<u8>) -> Result<()> {
        if let Err(e) = self.host_mpsc_tx.send(notification).await {
            return Err(Error::msg(format!("mpsc-receiver dropped: {}", e)));
        }

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
