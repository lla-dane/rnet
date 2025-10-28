use crate::peer_info::PeerInfo;
use std::collections::HashMap;
pub mod peer_info;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PeerData {
    pub peer_info: PeerInfo,
    pub peer_store: HashMap<String, PeerInfo>,
}