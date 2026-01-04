use anyhow::{Ok, Result};
use rnet_peer::peer_info::PeerInfo;
use rnet_tcp::TcpConn;
use rnet_traits::transport::Connection;
use rnet_traits::transport::SendReceive;

#[derive(Debug)]
pub struct RawConnection {
    pub stream: TcpConn,
    pub peer_info: PeerInfo,
    pub is_initiator: bool,
}

impl RawConnection {
    pub async fn read(&mut self) -> Result<Vec<u8>> {
        Ok(self.stream.recv_msg().await?)
    }

    pub async fn write(&mut self, msg: &Vec<u8>) -> Result<()> {
        Ok(self.stream.send_bytes(msg).await?)
    }

    pub async fn close(&mut self) -> Result<()> {
        Ok(self.stream.close().await?)
    }

    pub fn peer_info(&self) -> PeerInfo {
        self.peer_info.clone()
    }
}

#[cfg(test)]
mod tests {
    use prost::Message as ProstMessage;
    use rnet_proto::floodsub::{rpc::SubOpts, Message, Rpc};

    #[test]
    fn test_floodsub_rpc_serialization() {
        let mut rpc = Rpc::default();

        rpc.subscriptions.push(SubOpts {
            subscribe: Some(true),
            topic_id: Some("lib-chat".to_string()),
        });

        let msg = Message {
            from: Some(vec![1, 2, 3]),
            data: Some("hello world".as_bytes().to_vec()),
            seqno: Some(vec![0, 0, 0, 1]),
            topic_ids: vec!["lib-chat".to_string()],
        };
        rpc.publish.push(msg);

        let mut buf = Vec::new();
        rpc.encode(&mut buf).expect("Encoding failed");

        assert!(!buf.is_empty());
        println!("Serialized RPC size: {} bytes", buf.len());

        // 3. Deserialize back to Rpc struct
        let decoded_rpc = Rpc::decode(&buf[..]).expect("Decoding failed");

        // 4. Verify data integrity
        assert_eq!(decoded_rpc.subscriptions.len(), 1);
        assert_eq!(
            decoded_rpc.subscriptions[0].topic_id.as_ref().unwrap(),
            "lib-chat"
        );
        assert_eq!(decoded_rpc.subscriptions[0].subscribe, Some(true));

        assert_eq!(decoded_rpc.publish.len(), 1);
        let decoded_msg = &decoded_rpc.publish[0];
        assert_eq!(decoded_msg.data.as_ref().unwrap(), "hello world".as_bytes());
        assert_eq!(decoded_msg.topic_ids[0], "lib-chat");
    }
}
