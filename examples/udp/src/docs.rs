use anyhow::Result;
use tokio::net::UdpSocket;

pub async fn server() -> Result<()> {
    let socket = UdpSocket::bind("127.0.0.1:8080").await.unwrap();
    println!("UDP socket listening on: {:?}", socket.local_addr());
    let mut buf = [0; 1024];

    loop {
        let (len, addr) = socket.recv_from(&mut buf).await.unwrap();
        println!("{:?} bytes received from {:?}", len, addr);

        let len = socket.send_to(&buf[..len], addr).await?;
        println!("{:?} bytes sent", len);
    }
}

pub async fn client() -> Result<()> {
    let socket = UdpSocket::bind("127.0.0.1:9000").await.unwrap();
    println!("UDP socket listening on: {:?}", socket.local_addr());
    let mut buf = [0; 1024];
    let remote = "127.0.0.1:8080";

    let bytes = b"hello_from_udp_client";
    buf[0..bytes.len()].copy_from_slice(bytes);

    socket.send_to(bytes, remote).await.unwrap();
    let (len, addr) = socket.recv_from(&mut buf).await.unwrap();

    println!("received: {:?}, from {}", &buf[0..bytes.len()], addr);

    Ok(())
}
