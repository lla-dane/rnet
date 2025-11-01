use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;

use anyhow::{Ok, Result};
use rnet_multiaddr::{Multiaddr, Protocol};
use rnet_peer::{peer_info::PeerInfo, PeerData};
use rnet_tcp::{TcpConn, TcpTransport};
use rnet_traits::transport::Transport;
use tracing::{debug, info};

use crate::{
    keys::rsa::RsaKeyPair,
    multiselect::{multiselect::Multiselect, mutilselect_com::MultiselectComm},
    RawConnection,
};

#[derive(Debug)]
pub struct BasicHost {
    pub transport: TcpTransport,
    pub key_pair: RsaKeyPair,
    pub peer_data: Arc<Mutex<PeerData>>,
    pub connections: Arc<Mutex<HashMap<String, Arc<Mutex<RawConnection>>>>>,
    pub stream_handlers: HashMap<String, String>,
    pub multiselect: Multiselect,
    pub multiselect_comm: MultiselectComm,
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

        // ----------------------------

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

    pub async fn dial(self: &Arc<Self>, addr: &Multiaddr) -> Result<()> {
        let stream = TcpTransport::dial(addr).await?;
        self.conn_handler(stream, true).await.unwrap();

        Ok(())
    }

    pub async fn conn_handler(self: &Arc<Self>, stream: TcpConn, is_initiator: bool) -> Result<()> {
        let local_peer_info = {
            let data = self.peer_data.lock().await;
            data.peer_info.clone()
        };

        // security upgrade
        // Identify \/
        // stream_multiplexer upgrade
        // Active protocol negotiation

        let raw_conn = if !is_initiator {
            self.multiselect.handshake(&local_peer_info, stream).await?
        } else {
            self.multiselect_comm
                .handshake(&local_peer_info, stream)
                .await?
        };

        let peer_info = {
            let conn = raw_conn.lock().await;
            conn.peer_info().clone()
        };

        {
            let mut peer_data = self.peer_data.lock().await;
            peer_data
                .peer_store
                .insert(peer_info.peer_id.clone(), peer_info.clone());

            let mut connections = self.connections.lock().await;
            connections.insert(peer_info.peer_id.clone(), raw_conn.clone());
        }

        info!("New peer connected: {}", peer_info.peer_id);

        let host = self.clone();
        tokio::spawn(async move {
            host.handle_incoming(raw_conn, peer_info.peer_id.as_str())
                .await
                .unwrap();
        });

        Ok(())
    }
    async fn handle_incoming(
        self: &Arc<Self>,
        raw_conn: Arc<Mutex<RawConnection>>,
        peer_id: &str,
    ) -> Result<()> {
        loop {
            let mut conn = raw_conn.lock().await;
            match conn.read().await {
                std::result::Result::Ok(buf) => {
                    if buf.is_empty() {
                        break;
                    }

                    info!("Received a msg");
                }

                Err(err) => {
                    debug!("read error from {}: {:?}", peer_id, err);
                    break;
                }
            }
        }

        self.on_disconnect(&peer_id).await?;
        Ok(())
    }

    async fn on_disconnect(self: &Arc<Self>, peer_id: &str) -> Result<()> {
        debug!("Peer disconnected: {}", peer_id);

        let mut peer_data = self.peer_data.lock().await;
        peer_data.peer_store.remove(peer_id);

        let mut connections = self.connections.lock().await;
        connections.remove(peer_id);

        Ok(())
    }
}
