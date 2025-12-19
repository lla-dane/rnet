# TCP Transport

This module provides a TCP-based implementation of the Transport and Connection traits used throughout rnet.

It is responsible for establishing and managing raw network connections and exposes a minimal, async-friendly API for reading and writing bytes.

## What it implements

**`TcpTransport`**: A thin wrapper around `tokio::net::TcpListener` and implements the `Transport` trait.

It supports:
- listening on a TCP address derived from a Multiaddr
- accepting incoming connections
- dialing remote peers using TCP

The transport layer does not interpret data and does not provide stream multiplexing.

**`TcpConn`**: It wraps a `tokio::net::TcpStream` and represents a single TCP connection.

It implements:
- `Connection` — basic async read, write, and close operations
- `SendReceive` — a simple length-prefixed message framing helper

The framing format is:
- 4-byte big-endian message length
- followed by the raw message bytes

