use std::{collections::HashMap, sync::Arc};

use crate::upgrader::ConnUpgrader;
use anyhow::Result;
use identity::multiaddr::Multiaddr;
use muxer::mplex::conn::AsyncHandler;
use tokio::sync::{
    mpsc::{Receiver, Sender},
    Mutex,
};
use transport::transport::Transport;

type ConnectionMap = HashMap<String, Sender<Vec<u8>>>;

pub struct Swarm {
    pub transport: Transport,
    pub upgrader: ConnUpgrader,
    pub connections: Arc<Mutex<ConnectionMap>>,
    pub handlers: HashMap<String, AsyncHandler>,
    pub host_mpsc_rx: Receiver<Vec<u8>>,
}

// get_peer_id
// set_stream_handler
// get_connections
// get_total_connections
// dial_peer
// upgrade_outbound
// upgrade_inbound
// new_stream
// listen
// close
// close_peer
// add_conn
// notifications/notifiee all
impl Swarm {
    pub async fn new(
        transport_opt: &str,
        listen_addr: &mut Multiaddr,
        handlers: HashMap<String, AsyncHandler>,
        host_mpsc_rx: Receiver<Vec<u8>>,
    ) -> Result<Self> {
        let transport = Transport::new(transport_opt, listen_addr).await.unwrap();

        Ok(Swarm {
            transport,
            upgrader: ConnUpgrader::new(),
            connections: Arc::new(Mutex::new(HashMap::new())),
            handlers,
            host_mpsc_rx,
        })
    }

    pub async fn initiate(&self) -> Result<()> {
        Ok(())
    }
}
