# rnet -- A minimal P2P networking stack in Rust

This repository is an experimental peer-to-peer networking library written in Rust. The goal is for me to understand how a P2P stack is built from ground up -- especially the complex RUST async patterns involved when we combine transports, multiplexing, and application-level protocols.

### A p2p chat-room over floodsub using rnet

https://github.com/user-attachments/assets/2dfd2f38-75fe-454e-a003-225765f63c20

## Repository Structure

The main components of this repository are structured as follows:

- `core/`: Foundational types and traits used across the entire stack.

  This crate defines:

  - peer identity and peer-related data structures.
  - multiaddr parsing and representation.
  - core traits that describe transports, connections, and stream abstractions.
    Almost every other crate depend on `core`, moke it the conceptual base of the system.

- `transport/`: Implementation of connection level networking

  Curretly includes:

  - **TCP transport**, responsible for establishing raw byte streams between peers.

- `muxer/`: Stream multiplexing implementations build on top of transports.

  Currently includes:

  - **mplex**, allowing multiple logical streams over a single transport connection
    Muxers sit between the transport and protocol layers and are responsible for stream lifecycle management, framing and backpressure.

- `protocols/`: Implimentation of application-level protocols running over multiplexed streams.

  Currently includes:

  - **identify**, peer indentification handshake
  - **floodsub**, an end-to-end pubsub router with peer-subscriptions, gated propagation, dedup with last-seen message cache.

- `examples/`: Runnable binaries demonstrating how to assemble the stack end-to-end.

  Examples show:

  - how to create a host
  - how to listen or dial using multiaddrs
  - how transports, muxers, and protcols are wired together in practice.
    These are intended as executable reference points, not just sample snippets.

---

> Legend: ✅: Done  🛠️: In Progress/Usable  🌱 Prototype/Unstable  ❌: Missing

## Transports

| **Transport**  | **Status** |                              **Source**                              |
| -------------- | :--------: | :------------------------------------------------------------------: |
| **`rnet-tcp`** |     ✅     | [source](https://github.com/lla-dane/rnet/tree/master/transport/tcp) |

---

### Secure Communication

| **Secure Communication** | **Status** | **Source** |
| ------------------------ | :--------: | :--------: |
| **`rnet-noise`**         |     🛠️     |            |

---

### Stream Muxers

| **Stream Muxers** | **Status** |                             **Source**                             |
| ----------------- | :--------: | :----------------------------------------------------------------: |
| **`rnet-mplex`**  |     ✅     | [source](https://github.com/lla-dane/rnet/tree/master/muxer/mplex) |

---

### Core-Modules

| **Stream Muxers**    | **Status** |                             **Source**                                    |
| -----------------    | :--------: | :-----------------------------------------------------------------------: |
| **`rnet-floodsub`**  |     ✅     | [source](https://github.com/lla-dane/rnet/tree/master/protocols/floodsub) |

---

### General Purpose Utilities & Datatypes

| **Utility/Datatype** | **Status** |                                **Source**                                 |
| -------------------- | :--------: | :-----------------------------------------------------------------------: |
| **`rnet-ping`**      |     ✅     |   [source](https://github.com/lla-dane/rnet/tree/master/examples/ping)    |
| **`rnet-identify`**  |     ✅     | [source](https://github.com/lla-dane/rnet/tree/master/protocols/identify) |

## Explanation of Basic Two Node Communication

### Core Concepts

_(non-normative, useful for understanding, not a reference)_

Several components of a p2p stack take part when establishing a connection between two nodes:

1. **Host**: a node in a p2p network.
1. **Connection**: the layer 3 connection between two nodes in a p2p network.
1. **Transport**: the component that creates a _Connection_, e.g. TCP, UDP, QUIC, etc.
1. **Streams**: an abstraction on top of a _Connection_ representing parallel conversations about different matters, each of which is identified by a protocol ID. Multiple streams are layered on top of a _Connection_ via the _StreamMultiplexer_. e.g. MPLEX, YAMUX, etc.
1. **Multiplexer**: a component that is responsible for wrapping messages sent on a stream with an envelope that identifies the stream they pertain to, normally via an ID. The multiplexer on the other unwraps the message and routes it internally based on the stream identification.
1. **Secure channel**: optionally establishes a secure, encrypted, and authenticated channel over the _Connection_. e.g. NOISE, TLS
1. **Upgrader**: a component that takes a raw layer 3 connection returned by the _Transport_, and performs the security and multiplexing negotiation to set up a secure, multiplexed channel on top of which _Streams_ can be opened.

### Communication between two hosts X and Y

_(non-normative, useful for understanding, not a reference)_

**Initiate the connection**: A host is simply a node in a p2p network that is able to communicate with other nodes in the network. In order for X and Y to communicate with one another, one of the hosts must initiate the connection. Let's say that X is going to initiate the connection. X will first open a connection to Y. This connection is where all of the actual communication will take place.

**Communication over one connection with multiple protocols**: X and Y can communicate over the same connection using different protocols and the multiplexer will appropriately route messages for a given protocol to a particular handler function for that protocol, which allows for each host to handle different protocols with separate functions. Furthermore, we can use multiple streams for a given protocol that allow for the same protocol and same underlying connection to be used for communication about separate topics between nodes X and Y.

**Why use multiple streams?**: The purpose of using the same connection for multiple streams to communicate over is to avoid the overhead of having multiple connections between X and Y. In order for X and Y to differentiate between messages on different streams and different protocols, a multiplexer is used to encode the messages when a message will be sent and decode a message when a message is received. The multiplexer encodes the message by adding a header to the beginning of any message to be sent that contains the stream id (along with some other info). Then, the message is sent across the raw connection and the receiving host will use its multiplexer to decode the message, i.e. determine which stream id the message should be routed to.

_This structure is inspired by libp2p's layering model. but implemented independently to better understand the mechanics and async patterns involved._
