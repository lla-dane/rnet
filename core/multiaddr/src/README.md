# Multiaddr

This module provides a minimal implementation of multiaddresses used throughout rnet to describe how peers can be reached.

A multiaddr represents a network address as an ordered list of protocol components (e.g. IP, transport, peer ID), rather than a single opaque string.

## Overview

A `Multiaddr` is parsed from a string such as:
```sh
/ip4/127.0.0.1/tcp/8080/p2p/afopponnfsklngllsbgjzafnafangg
```

Internally, it is represented as a sequence of protocol components that can be inspected, modified, or extended by higher layers.

This makes address handling explicit and avoids hard-coding assumptions about transports or protocols.


