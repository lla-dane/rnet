# Examples

A set of examples showcasing how to use rnet.

## Getting started

To run any example in this directory, you can use Cargo:

```sh
# Get to root directory
cargo run --bin ping --release
```

Each example includes its own README.md file with specific instructions on how to use it. Most examples require running multiple instances to demonstrate peer-to-peer communication, so be sure to read the individual example documentation.

### Prerequisites

- Rust and Cargo installed (see [rustup.rs](https://rustup.rs/) for installation)
- Basic understanding of peer-to-peer networking concepts
- Some examples may require additional dependencies specific to their functionality

## Individual libp2p features

- [Ping](./ping): Small `ping` clone, sending a ping to a peer, expecting a pong as a response.
- [TCP](./tcp): A minimal TCP client/server example built on `TcpTransport`, illustrating async accept loops, task spawning, and connection I/O.
