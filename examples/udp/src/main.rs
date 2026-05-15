use std::env;

use identity::multiaddr::Multiaddr;
use tracing::info;
use tracing_subscriber::EnvFilter;
use transport::transport::Transport;
pub mod docs;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new("trace"))
        .without_time()
        .with_target(false)
        .compact()
        .init();
    let args: Vec<String> = env::args().collect();

    let mut mode = "server".to_string();
    let mut destination = "";
    if args.len() > 1 {
        mode = "client".to_string();
        destination = &args[1];
    }

    if mode == "server" {
        let mut listen_addr = Multiaddr::new(
            "ip4/127.0.0.1/udp/8000/p2p/7627rBpXchM2Q4PE6iBVKeRFkE2MQRM2k1qn94cgdsgR",
        )
        .unwrap();

        let (transport, _) = Transport::new("udp", &mut listen_addr).await.unwrap();
        tokio::spawn(async move {
            transport.accept().await.unwrap();
        });

        info!(
            "Run in new terminal: \ncargo run --bin udp {:?}",
            listen_addr.to_string()
        );
    } else {
        let mut listen_addr = Multiaddr::new(
            "ip4/127.0.0.1/udp/9000/p2p/7627rBpXchM2Q4PE6iBVKeRFkE2MQRM2k1qn94aiksgI",
        )
        .unwrap();
        let multiaddr = Multiaddr::new(destination).unwrap();
        let (transport, _) = Transport::new("udp", &mut listen_addr).await.unwrap();

        let _udp_conn = transport.dial(&multiaddr).await.unwrap();
    }
    loop {}
}
