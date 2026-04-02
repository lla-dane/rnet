use std::{collections::HashMap, sync::Arc};

use rnet_muxer::mplex::conn::AsyncHandler;
use rnet_traits::{core::IReadWriteClose, transport::ITransport};
use tokio::sync::{
    mpsc::{Receiver, Sender},
    Mutex,
};

use crate::{basic_host::HostMpscTx, upgrader::TransportUpgrader};

type ConnectionMap = HashMap<String, Sender<Vec<u8>>>;

pub struct Swarm<T>
where
    T: ITransport<Box<dyn IReadWriteClose + 'static>>,
{
    pub transport: T,
    pub upgrader: TransportUpgrader,
    pub connections: Arc<Mutex<ConnectionMap>>,
    pub handlers: HashMap<String, AsyncHandler>,
    pub host_mpsc_tx: Arc<HostMpscTx>,
    pub host_mpsc_rx: Receiver<Vec<u8>>,
}
