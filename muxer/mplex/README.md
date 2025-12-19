# Mplex Stream Multiplexer

This module implements a lightweight stream multiplexing layer inspired by mplex.

It allows multiple logical streams to be created and managed over a single underlying transport connection, enabling concurrent protocol execution without opening multiple TCP connections.

## Overview

The mplex layer sits between the transport and protocol layers.

It is responsible for:
- framing data into stream-specific messages
- creating and closing logical streams
- routing incoming frames to the correct stream
- dispatching streams to protocol handlers after a simple handshake

Each multiplexed stream behaves like an independent, bidirectional channel, while sharing a single underlying connection.

## Core components

- **`MuxedConn`**: This represents a multiplexed connection built on top of a raw transport connection.

    It manages:
    - the underlying RawConnection
    - stream ID allocation and tracking
    - a map of active streams and their message channels
    - demultiplexing of incoming frames
    - forwarding payloads to the appropriate `MuxedStream`

    Incoming frames are parsed, classified by flag and stream ID, and routed accordingly.

- **`MuxedStream`**: This represents a single logical stream within a multiplexed connection.

    It provides:
    - async read and write operations
    - stream-scoped protocol negotiation
    - access to remote peer information
    - execution of protocol-specific async handlers

    Each stream is independent and can run concurrently with others.

### Framing format

Frames are encoded as:
- varint-encoded header containing stream ID and message flag
- varint-encoded payload length
- raw payload bytes

The header embeds both the stream identifier and the message type, allowing efficient demultiplexing.

### Protocol handshake

When a new stream is created, a minimal handshake is performed:
- the initiator sends the protocol name as the first message
- the responder validates the protocol against registered handlers
- both sides confirm the protocol before executing the handler

This mechanism provides basic protocol dispatch without a full negotiation framework.