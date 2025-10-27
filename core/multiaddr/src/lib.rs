use std::{
    fmt,
    net::{Ipv4Addr, Ipv6Addr},
};

use anyhow::{bail, Ok, Result};

#[derive(Debug, Clone)]
pub struct Multiaddr {
    _addr: String,
    pub components: Vec<Protocol>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Protocol {
    Ip4(Ipv4Addr),
    Ip6(Ipv6Addr),
    Tcp(u16),
    Udp(u16),
    P2P(String),
}

impl fmt::Display for Protocol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Protocol::Ip4(ip) => write!(f, "/ip4/{}", ip),
            Protocol::Ip6(ip) => write!(f, "/ip6/{}", ip),
            Protocol::Tcp(port) => write!(f, "/tcp/{}", port),
            Protocol::Udp(port) => write!(f, "/udp/{}", port),
            Protocol::P2P(peer_id) => write!(f, "/p2p/{}", peer_id),
        }
    }
}

/// Example: ip4/127.0.0.1/tcp/8080/p2p/afopponnfsklngllsbgjzafnafangg
impl Multiaddr {
    pub fn new(addr: &str) -> Result<Self> {
        // Validate the addr
        let components = Self::parse(addr).unwrap();
        Ok(Self {
            _addr: String::from(addr),
            components,
        })
    }

    fn parse(addr: &str) -> Result<Vec<Protocol>> {
        let mut components = Vec::new();
        let parts: Vec<&str> = addr.trim_matches('/').split('/').collect();

        let mut i = 0;
        while i < parts.len() {
            match parts[i] {
                "ip4" => {
                    let ip: Ipv4Addr = parts
                        .get(i + 1)
                        .expect("Missing IPv4 value")
                        .parse()
                        .unwrap();
                    components.push(Protocol::Ip4(ip));
                    i += 2;
                }
                "ip6" => {
                    let ip: Ipv6Addr = parts
                        .get(i + 1)
                        .expect("Missing IPv6 value")
                        .parse()
                        .unwrap();
                    components.push(Protocol::Ip6(ip));
                    i += 2;
                }
                "tcp" => {
                    let port: u16 = parts.get(i + 1).expect("missing TCP port").parse().unwrap();
                    components.push(Protocol::Tcp(port));
                    i += 2;
                }
                "udp" => {
                    let port: u16 = parts
                        .get(i + 1)
                        .ok_or_else(|| anyhow::anyhow!("missing UDP port"))?
                        .parse()?;
                    components.push(Protocol::Udp(port));
                    i += 2;
                }
                "p2p" => {
                    let peer_id = parts
                        .get(i + 1)
                        .ok_or_else(|| anyhow::anyhow!("missing P2P id"))?
                        .to_string();
                    components.push(Protocol::P2P(peer_id));
                    i += 2;
                }
                p => bail!("Unkown protocol: {}", p),
            }
        }

        Ok(components)
    }

    pub fn value_for_protocol(&self, proto: &str) -> Option<String> {
        for protocol in &self.components {
            match (proto, protocol) {
                ("ip4", Protocol::Ip4(ip)) => return Some(ip.to_string()),
                ("ip6", Protocol::Ip6(ip)) => return Some(ip.to_string()),
                ("tcp", Protocol::Tcp(port)) => return Some(port.to_string()),
                ("udp", Protocol::Udp(port)) => return Some(port.to_string()),
                ("p2p", Protocol::P2P(peer_id)) => return Some(peer_id.to_string()),
                _ => continue,
            }
        }
        None
    }

    pub fn replace_value_for_protocol(&mut self, proto: &str, value: &str) -> Result<()> {
        for protocol in &mut self.components {
            match (proto, protocol) {
                ("ip4", Protocol::Ip4(ip)) => {
                    *ip = value
                        .parse()
                        .map_err(|_| anyhow::anyhow!("Invalid IPv4 address"))?;
                }
                ("ip6", Protocol::Ip6(ip)) => {
                    *ip = value
                        .parse()
                        .map_err(|_| anyhow::anyhow!("Invalid IPv6 address"))?;
                    return Ok(());
                }
                ("tcp", Protocol::Tcp(port)) => {
                    *port = value
                        .parse()
                        .map_err(|_| anyhow::anyhow!("Invalid TCP port"))?;
                    return Ok(());
                }
                ("udp", Protocol::Udp(port)) => {
                    *port = value
                        .parse()
                        .map_err(|_| anyhow::anyhow!("Invalid UDP port"))?;
                    return Ok(());
                }
                ("p2p", Protocol::P2P(peer_id)) => {
                    *peer_id = value.to_string();
                    return Ok(());
                }
                _ => continue,
            }
        }

        Ok(())
    }

    pub fn push_proto(&mut self, proto: Protocol) {
        self.components.push(proto);
    }

    pub fn to_string(&self) -> String {
        self.components
            .iter()
            .map(|c| c.to_string())
            .collect::<Vec<_>>()
            .join("")
    }
    pub fn push(&mut self, proto: Protocol) {
        self.components.push(proto);
    }

    pub fn pop(&mut self) -> Option<Protocol> {
        self.components.pop()
    }
}
