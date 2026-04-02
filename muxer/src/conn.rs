use std::{collections::HashMap, sync::Arc};

use anyhow::{Error, Result};
use rnet_peer::peer_info::PeerInfo;
use rnet_traits::{
    conn::{IMuxedConn, IRawConnection},
    host::IHostMpscTx,
};
use tokio::sync::mpsc::{Receiver, Sender};

use crate::mplex::conn::{AsyncHandler, MplexConn};

// mplex-conn: IMuxedConn
// handle_incoming
// conn_handler
// write

// mplex-stream: IMuxedStream
// write
// read
// server_handshake
// client_handshake
pub struct MuxedConn {
    conn: Box<dyn IMuxedConn>,
    pub is_initiator: bool,
    pub remote_peer: PeerInfo,
    pub muxed_mpsc_tx: Sender<Vec<u8>>,
}

// TODO: in future if we want a unified conn-handler in MuxedConn itself,
// then `MuxedConn[raw_conn] + mplex/yamux mpsc channels` in which the routing
// will be from MuxedConn, but the decision making will be in internal
// multiplexing router i.e mplex / yamux
impl MuxedConn {
    pub fn new<W>(
        protocol: &str,
        raw_conn: W,
        is_initiator: bool,
        remote_peer: PeerInfo,
        handlers: HashMap<String, AsyncHandler>,
        muxed_mpsc_rx: Receiver<Vec<u8>>,
        muxed_mpsc_tx: Sender<Vec<u8>>,
    ) -> Result<Self>
    where
        W: IRawConnection + Send + Sync + 'static,
    {
        match protocol {
            "mplex" => {
                let mplex_conn = MplexConn::new(
                    raw_conn,
                    is_initiator,
                    remote_peer.clone(),
                    handlers,
                    muxed_mpsc_tx.clone(),
                    muxed_mpsc_rx,
                );

                Ok(MuxedConn {
                    conn: Box::new(mplex_conn),
                    is_initiator,
                    remote_peer,
                    muxed_mpsc_tx,
                })
            }
            _ => Err(Error::msg("protocol not found")),
        }
    }

    pub async fn conn_handler(
        mut self,
        peer_id: &str,
        host_mpsc_tx: Arc<dyn IHostMpscTx + Send + Sync + 'static>,
    ) -> Result<()> {
        let peer_id = peer_id.to_string();
        tokio::spawn(async move {
            self.conn
                .conn_handler(&peer_id, host_mpsc_tx)
                .await
                .unwrap();
        });

        Ok(())
    }
}
