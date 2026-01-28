- For continuous read/write operability on TcpStreams, used the tokio::select! pattern to do execute whatever Future gets complete and rerun the loop.

- In floodsub handle_api impl, moved around the Arc<Mutex<_>> of FloodsubPeers to conserve the &self consumption of the function so we can use the floodsub instance even after starting the handle_api tokio::spawned-task, in the application code. In contrast to this, we cannot use th host-instance after starting the BasicHost::run().

- Interior mutability over &mut self: In the Floosub API methods like `handle_dead_peers(&self, ...)` we can take a mut reference to something like `self.floodsub_store` if it is guarded inside shared state `Arc<Mutex<_>>`. This is possible via runtime-enforced interior mutability rather than exclusive compile-time borrowing.

- Needed the traits: `Hash, PartialEq, Eq` for inserting custom key-type in HashMaps 

- Stack: fixed-size data known at compile time, ownership is clear and local. Heap, a big pool of memory managed at runtime, allocation requires taking to an allocator, lifetime is not tied to a scope by default. In heap, there is runtime overhead, only during allocation.

- Generic & trait bounds Vs trait objects: Only one object-type allowed when using generic trait bounds, but can use multiple object-type satisfying the trait in trait objects. Generic trait bounds are more performant.

- non-generic impl/traits, we have to use concrete type of structs, whose types are defined at runtime. But with generic impl/traits, we can use structs with generic type declaration, like `Foo<f32>` and `Foo<char>` can both use `Value<T>` trait.

- use trait-generics for non-associated types, and for associated type, declare them inside the trait declaration.

- Everytime we use `#[async_trait]`, the async methods return `Send` futures by default. If everything inside the async method is already `Send` -> we won't notice. But if anything captured is not provably `Send` -> the compiler will force

- `Send` is only required when an async future is created and promised to be movable across threads, the promise is generally made by `#[async_trait]`, but we can also opt-out of it. In Rnet infra, I doubt I moved the TcpStreams to multiple threads.

- 
