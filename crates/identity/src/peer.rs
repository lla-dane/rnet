use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PeerInfo {
    pub peer_id: String,
    pub listen_addr: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PeerData {
    pub peer_info: PeerInfo,
    pub peer_store: HashMap<String, PeerInfo>,
}
