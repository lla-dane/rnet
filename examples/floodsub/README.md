## Description

This example showcases how to create a p2p chat-room using floodsub as the message broadcasting protcol

## Demo

https://github.com/user-attachments/assets/479fe257-331a-4492-87a9-2c11195d1727

## Usage

To run the example, follow these steps:

1. In a first terminal window, run the following command:

   ```sh
   cargo run --bin floodsub --release
   ```

   This command starts a node and initialises a Floodsub CLI interface to create a p2p chat-room:

   ```bash
   FLOODSUB CLI ready. Commands:
      help                       => print all the commands
      local                      => get local peer-info
      connect <maddr>            => connect with a new peer
      new_stream <maddr>         => open a new floodsub stream with the peer
      sub <topic>                => subscribe to a new-topic
      unsub <topic>              => unsubscribe to a new-topic
      pub <topic> <msg>          => publish a msg to a topic
      topics                     => list the subscribed topics
      peers                      => list the connected peers
      mesh                       => map of topics -> peer

   Command => DEBUG Floodsub message cache cleanup
   ```

2. In a second terminal window, start a new instance of the node with the following command:

   ```sh
   cargo run --bin floodsub --release /ip4/127.0.0.1/tcp/44389/p2p/2958bs....
   ```

   Replace `/ip4/127.0.0.1/tcp/44389` with the listen address of the first node obtained from the first terminal window.

3. The two nodes have an connected tcp stream with the floodsub stream-handler, now follow the demo video above to create a chat-room using the cli, and play-around.

## Conclusion

The floodsub example demonstrates how `rnet` can be used to build a simple p2p `publish–subscribe` system on top of raw peer connections.
By running multiple nodes and interacting through the CLI, users can observe how topics, peer meshes, and message broadcasting work in practice, and gain a clearer understanding of Floodsub’s behavior and limitations in a real p2p setting.
