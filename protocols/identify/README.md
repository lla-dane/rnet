# Identify Protocol

The identify module implements a minimal peer identification exchange.

Its purpose is to allow two connected peers to exchange basic identity information immediately after a connection is established, before any higher-level protocols are run.

This is a foundational step in the connection lifecycle, as many protocols need to know who they are talking to before proceeding.