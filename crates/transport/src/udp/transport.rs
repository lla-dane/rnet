use std::{collections::HashMap, net::SocketAddr, sync::Arc, time::Duration};

use anyhow::Result;
use async_trait::async_trait;
use identity::{multiaddr::Multiaddr, traits::transport::ITransport};
use tokio::{
    net::UdpSocket,
    sync::{
        mpsc::{self, Receiver, Sender},
        Mutex,
    },
};
use tracing::{error, info};

use crate::udp::{
    conn::UdpConn,
    headers::{deserialize_udp_packet, serialize_udp_packet, UdpPacketFlag},
};

type OutgoingFrame = (SocketAddr, Vec<u8>);
#[derive(Clone)]
pub struct UdpTransport {
    listener: Arc<UdpSocket>,
    write_req_tx: Sender<OutgoingFrame>,
    peerstore: Arc<Mutex<HashMap<SocketAddr, Sender<Vec<u8>>>>>,
    pub local_peer_id: String,

    // new peer notifications
    new_peer_tx: Sender<SocketAddr>,
    new_peer_rx: Arc<Mutex<Receiver<SocketAddr>>>,
}

impl UdpTransport {
    pub fn get_local_addr(&self) -> Result<String> {
        let addr = self.listener.local_addr().unwrap().to_string();
        Ok(addr)
    }

    pub async fn io_loop(&self, mut write_req_rx: Receiver<OutgoingFrame>) -> Result<()> {
        info!("Starting UDP io loop");
        let mut buffer = vec![0u8; 1500]; // typical MTU size
        loop {
            tokio::select! {
                Ok((len, remote_socket)) = self.listener.recv_from(&mut buffer) => {
                    let data = &buffer[0..len];

                    let (flag, payload) = match deserialize_udp_packet(data) {
                        Some((flag, payload)) => (flag, payload),
                        None => continue,
                    };

                    match flag {
                        UdpPacketFlag::Connect => {
                            info!("New Peer Connected: {}", String::from_utf8_lossy(&payload));
                            self.new_peer_tx.send(remote_socket).await.unwrap();
                        },
                        UdpPacketFlag::Disconnect => {
                            // update the peerstore, clean the udp_Conn
                        },
                        UdpPacketFlag::General => {
                            // fetch the udp_conn and transmit the packet
                            let udp_conn_tx = {
                                let peerstore = self.peerstore.lock().await;
                                peerstore.get(&remote_socket).cloned()
                            };

                            udp_conn_tx.unwrap().send(payload).await.unwrap();
                        },
                    }
                }

                Some((socket, frame)) = write_req_rx.recv() => {
                    self.listener.send_to(&frame, socket).await.unwrap();
                }
            }
        }
    }
}

/// TODO: Shift this function to Multiaddr module
fn extract_socket_addr(addr: &Multiaddr) -> Result<(SocketAddr, String)> {
    let local_ip = addr.value_for_protocol("ip4").unwrap();
    let port = addr.value_for_protocol("udp").unwrap();
    let local_peer_id = addr.value_for_protocol("p2p").unwrap();

    let addr = format!("{}:{}", local_ip, port);

    Ok((addr.parse().unwrap(), local_peer_id))
}

#[async_trait]
impl ITransport<UdpConn> for UdpTransport {
    async fn listen(addr: &Multiaddr) -> Result<Self> {
        let (addr, local_peer_id) = extract_socket_addr(addr).unwrap();

        let listener = Arc::new(UdpSocket::bind(addr).await?);
        let peerstore = Arc::new(Mutex::new(HashMap::new()));
        let (write_req_tx, write_req_rx) = mpsc::channel::<OutgoingFrame>(100);
        let (new_peer_tx, new_peer_rx) = mpsc::channel::<SocketAddr>(100);

        let transport = Self {
            listener,
            peerstore,
            local_peer_id,
            write_req_tx,
            new_peer_tx,
            new_peer_rx: Arc::new(Mutex::new(new_peer_rx)),
        };

        let arc_transport = Arc::new(transport.clone());
        tokio::spawn(async move {
            arc_transport.io_loop(write_req_rx).await.unwrap();
        });
        tokio::time::sleep(Duration::from_millis(1000)).await;

        Ok(transport)
    }

    async fn accept(&self) -> Result<(UdpConn, SocketAddr)> {
        let mut rx = self.new_peer_rx.lock().await;

        let socket_addr = rx
            .recv()
            .await
            .ok_or_else(|| error!("UDP new-connection receive channel closed"))
            .unwrap();

        let (socket_mpsc_tx, socket_mpsc_rx) = mpsc::channel::<Vec<u8>>(100);

        {
            let mut peerstore = self.peerstore.lock().await;
            peerstore.insert(socket_addr, socket_mpsc_tx);
        }

        Ok((
            UdpConn {
                write_req_tx: self.write_req_tx.clone(),
                socket_mpsc_rx,
                remote_socket: socket_addr,
            },
            socket_addr,
        ))
    }

    async fn dial(&self, addr: &Multiaddr) -> Result<UdpConn> {
        let (remote_socket_addr, remote_peer_id) = extract_socket_addr(addr).unwrap();
        let local_peer_id = self.local_peer_id.as_bytes().to_vec();

        let packet = serialize_udp_packet(local_peer_id, UdpPacketFlag::Connect);

        self.write_req_tx
            .send((remote_socket_addr, packet))
            .await
            .unwrap();
        let (socket_mpsc_tx, socket_mpsc_rx) = mpsc::channel::<Vec<u8>>(100);

        {
            // update peerstore
            let mut peerstore = self.peerstore.lock().await;
            peerstore.insert(remote_socket_addr, socket_mpsc_tx);
        }

        info!("New peer connected: {}", remote_peer_id);

        Ok(UdpConn {
            write_req_tx: self.write_req_tx.clone(),
            socket_mpsc_rx,
            remote_socket: remote_socket_addr,
        })
    }
}
