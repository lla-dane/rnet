## Host

This module contains the core host runtime of an rnet node. This crate is basically about:

> "How does a node accept raw TCP connection and turn it into a multiplexed, identify-aware peer connection?"

The architecture includes:
- **BasicHost** -- the runtime and event loop
- **Multiselect** -- protocol negotiation and identify handshake
- **Keys** -- peer identity and message authentication

## Layout
```bash
host/
├── basic_host.rs        # Node runtime and connection lifecycle
├── headers.rs           # Internal host control framing
├── keys/
│   ├── mod.rs           # Key trait abstraction
│   └── rsa.rs           # RSA-based peer identity
├── multiselect/
│   ├── mod.rs
│   ├── multiselect.rs   # Initiator-side protocol negotiation
│   └── mutilselect_com.rs # Responder-side negotiation
└── lib.rs
```

- `BasicHost`: The main event loop of a node.
    It owns:
    - the TCP transport
    - active peer-connections and key-pairs
    - protocol stream handlers
    - an internal control channel

- `Multiselect`: This module handles protocol negotiation.
    There are 2 roles:
    - `Multiselect` -- used by the dialer (initiator)
    - `MultiselectConn` -- used by the listener (responder)

    Both negotiate:
    - multistream support
    - idenitfy protocol

    After negotiation the identify sequence run to exchange the `PeerInfo`

- `Keys`: This module defines the node identity.
    - `RsaKeyPair` is the default implementation

    Peer IDs are derived from the SHA-256 hash of the public key and encoded in base58. This makes identities deterministic and content-addressed.

