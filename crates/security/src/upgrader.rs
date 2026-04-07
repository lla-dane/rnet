use anyhow::Result;
use identity::traits::core::IReadWriteClose;

use crate::{conn::SecureConn, transport::SecureTransport};

pub struct SecurityUpgrader {
    tranport: SecureTransport,
}

impl Default for SecurityUpgrader {
    fn default() -> Self {
        Self::new()
    }
}

impl SecurityUpgrader {
    pub fn new() -> Self {
        let sec_opts = vec![String::from("dh")];
        SecurityUpgrader {
            tranport: SecureTransport { sec_opts },
        }
    }

    pub async fn update(
        &self,
        stream: Box<dyn IReadWriteClose + Send + Sync + 'static>,
        is_initiator: bool,
    ) -> Result<SecureConn> {
        Ok(self
            .tranport
            .secure_conn(stream, is_initiator)
            .await
            .unwrap())
    }
}
