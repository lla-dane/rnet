## Description

The TCP ping example demonstrates direct usage of the TcpTransport without any stream multiplexing or application-level protocols.

It shows how to:

- listen for incoming TCP connections
- dial a remote peer using a multiaddr
- perform basic asynchronous read and write operations over a raw connection

The example operates in a simple client/server mode and is intended as a low-level building block for understanding the transport layer of rnet.

## Usage

To run the example, follow these steps:

1. In a first terminal window, run the following command:

   ```sh
   cargo run --bin tcp --release
   ```

   This command starts a node and prints the listening-IP like this:

   ```sh
   Run in another terminal: cargo run -- 127.0.0.1:46749
   ```

2. In a second terminal window, start a new instance of the example with the following command:

   ```sh
   cargo run --bin tcp --release -- 127.0.0.1:46749
   ```

   Replace `127.0.0.1:46749` with the listening-IP of the first node obtained from the first terminal window.

3. The client sends a ping message to the server.
   The server receives the message, waits briefly, and responds with pong.

## Conclusion

The TCP ping example illustrates the most basic networking path in rnet: raw transport connections and asynchronous I/O.

It serves as a reference for how higher-level components such as stream multiplexers and protocols are layered on top of transports in the rest of the project.
