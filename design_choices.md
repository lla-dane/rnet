- For continuous read/write operability on TcpStreams, used the tokio::select! pattern to do execute whatever Future gets complete and rerun the loop.

- In floodsub handle_api impl, moved around the Arc<Mutex<_>> of FloodsubPeers to conserve the &self consumption of the function so we can use the floodsub instance even after starting the handle_api tokio::spawned-task, in the application code. In contrast to this, we cannot use th host-instance after starting the BasicHost::run().

- Interior mutability over &mut self: In the Floosub API methods like `handle_dead_peers(&self, ...)` we can take a mut reference to something like `self.floodsub_store` if it is guarded inside shared state `Arc<Mutex<_>>`. This is possible via runtime-enforced interior mutability rather than exclusive compile-time borrowing.

- Needed the traits: `Hash, PartialEq, Eq` for inserting custom key-type in HashMaps 