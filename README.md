# rnet -- A modular P2P networking stack in Rust

This repo contains an experimental, modular peer-to-peer networking library written in Rust. The primary objective is to dissect and understand hwo a complete p2p stack is built form the ground up. This involves navigating comples Rust async patterns to combine raw transports, secure channel upgrades, stream multiplexing, and application-level routing.

### Multiplexed Protocols: Floodsub & Ping over MPLEX
This latest iteration successfully demonstrates protocol multiplexing. Multiple logical streams (`ping` and `floodsub`) operate concurrently over a single underlying TCP transport connection.

https://github.com/user-attachments/assets/506826b9-cfb6-4947-90b0-d5c21dabd555

### Workplace Architecture
The monolithic structure has been refactored into a Cargo workspace. This isolates distinct networking phases into dedicated crates, allowing independent testing and trait based composition.

#### How is the repo structured?
The core components are broken down under the `crates/` directory:
- `identity/`: The foundational layer, it defines cryptographic keys `(RSA, X25519)`, `PeerId` generation, and `Multiaddr` parsing. Crucially, it houses the core stack traits (`transport, security, muxer, protocols`) that dictate how the higher-level crates interact.

- `transport/`: Connection establishment, currently implements the `tcp` transport, responsible for `dialing` and `listening` for raw byte streams.

- `security/`: The encryption phase, acts as an upgrader. It takes a raw layer 3 connection and performs a `Diffie-Hellman` key exchange to establish an encrypted, authenticated channel.

- `muxer/`: Stream multiplexing, implements `mplex`(for now). It wraps payloads in stream-identifying headers, allowing multiple distinct conversations (streams) to share the single secure connection. It handles framing, stream lifecycle, and backpressure.

- `multistream/`: Protocol negotiation, implements multiselect. Before a stream can be used, both peers must agree on what protocol will be spoken over it (e.g., /rnet/ping/1.0.0).

- `swarm/` & `node/`: Network management, `swarm` maintains the network state, pooling active connections and routing incoming streams. `node` acts as the high-level facade orchestrating the transport, upgrader, and active protocols.

- `schema/`: Message definitions. houses Protocol Buffer definitions `(.proto)` and auto-generates the Rust serialization types via `build.rs`.

- `protocols/`: Application-level logic, the actual distributed behaviors. Currently includes `ping` (liveness checks) and `floodsub` (a pub/sub router with peer-subscriptions, gated propagation, and a message deduplication cache).

### Communication between two hosts X and Y

_(non-normative, useful for understanding, not a reference)_

**Initiate the connection**: A host is simply a node in a p2p network that is able to communicate with other nodes in the network. In order for X and Y to communicate with one another, one of the hosts must initiate the connection. Let's say that X is going to initiate the connection. X will first open a connection to Y. This connection is where all of the actual communication will take place.

**Communication over one connection with multiple protocols**: X and Y can communicate over the same connection using different protocols and the multiplexer will appropriately route messages for a given protocol to a particular handler function for that protocol, which allows for each host to handle different protocols with separate functions. Furthermore, we can use multiple streams for a given protocol that allow for the same protocol and same underlying connection to be used for communication about separate topics between nodes X and Y.

**Why use multiple streams?**: The purpose of using the same connection for multiple streams to communicate over is to avoid the overhead of having multiple connections between X and Y. In order for X and Y to differentiate between messages on different streams and different protocols, a multiplexer is used to encode the messages when a message will be sent and decode a message when a message is received. The multiplexer encodes the message by adding a header to the beginning of any message to be sent that contains the stream id (along with some other info). Then, the message is sent across the raw connection and the receiving host will use its multiplexer to decode the message, i.e. determine which stream id the message should be routed to.

_This structure is inspired by libp2p's layering model. but implemented independently to better understand the mechanics and async patterns involved._

__*PS: The internal README.md files maybe a bit outdated.*__