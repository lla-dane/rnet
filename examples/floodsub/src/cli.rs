use anyhow::Result;
use rnet_floodsub::{
    pubsub::FloodSub,
    subscription::{build_floodsub_api_frame, SubAPIMpscFlag},
};
use rnet_host::basic_host::HostMpscTx;
use rnet_multiaddr::Multiaddr;
use std::{io::Write, sync::Arc, time::Duration};
use tokio::io::{self, AsyncBufReadExt};

use crate::{receiver_loop, FLOODSUB};

const CLI_DELAY: Duration = Duration::from_nanos(1000);
const COMMANDS: &[&str] = &[
    "help                       => print all the commands",
    "local                      => get local peer-info",
    "connect <maddr>            => connect with a new peer",
    "new_stream <maddr>         => open a new floodsub stream with the peer",
    "sub <topic>                => subscribe to a new-topic",
    "unsub <topic>              => unsubscribe to a new-topic",
    "pub <topic> <msg>          => publish a msg to a topic",
    "topics                     => list the subscribed topics",
    "peers                      => list the connected peers",
    "mesh                       => map of topics -> peer",
];

fn print_commands() {
    for cmd in COMMANDS {
        println!("      {}", cmd);
    }
}

async fn handle_cmd(line: &str, host_tx: &Arc<HostMpscTx>, floodsub: &Arc<FloodSub>) -> Result<()> {
    let mut parts = line.split_whitespace();
    let cmd = parts.next().unwrap();

    match cmd {
        "help" => {
            print_commands();
        }

        "local" => {
            let peer_info = floodsub.get_local().unwrap();
            println!("{}", peer_info.listen_addr);
        }

        "connect" => {
            let maddr = Multiaddr::new(parts.next().unwrap()).unwrap();
            host_tx.connect(&maddr).await?;
            tokio::time::sleep(Duration::from_millis(300)).await;
        }

        "new_stream" => {
            let maddr = parts.next().unwrap();
            host_tx
                .new_stream(maddr, vec![FLOODSUB.to_string()])
                .await
                .unwrap();
            tokio::time::sleep(Duration::from_millis(300)).await;
        }

        "sub" => {
            let topic = parts.next().unwrap().to_string();
            let sub_api = floodsub.subscribe(topic).await.unwrap();
            tokio::spawn(async move {
                receiver_loop(sub_api).await;
            });
        }

        "unsub" => {
            let topic = parts.next().unwrap().to_string();
            floodsub.unsubscribe(vec![topic]).await.unwrap();
        }

        "pub" => {
            let topic = parts.next().unwrap().to_string();
            let msg = parts.collect::<Vec<_>>().join(" ").as_bytes().to_vec();

            let frame = build_floodsub_api_frame(
                SubAPIMpscFlag::Publish,
                None,
                Some(vec![topic]),
                Some(msg),
            );

            floodsub.floodsub_mpsc_tx.send(frame).await.unwrap();
        }

        "topics" => match floodsub.get_subscribed_topics().await {
            Some(topics) => println!("{:?}", topics),
            None => println!("None"),
        },

        "peers" => match floodsub.get_connected_peers().await {
            Some(peers) => {
                for peer in peers {
                    println!("{}", peer);
                }
            }
            None => println!("None"),
        },

        "mesh" => {
            let mesh = floodsub.get_floodsub_mesh().await;

            if mesh.is_empty() {
                println!("None");
                return Ok(());
            }

            for (topic, peers) in mesh {
                println!("[{}] => {:?}", topic, peers);
            }
        }

        _ => {
            println!("unknown command");
        }
    }
    Ok(())
}

pub async fn cli_loop(host_tx: Arc<HostMpscTx>, floodsub: Arc<FloodSub>) -> Result<()> {
    let stdin = io::BufReader::new(io::stdin());
    let mut lines = stdin.lines();

    println!("\nFLOODSUB CLI ready. Commands:");
    print_commands();

    loop {
        print!("\nCommand => ");
        std::io::stdout().flush().unwrap();

        let line = match lines.next_line().await? {
            Some(line) => line,
            None => break,
        };

        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        handle_cmd(line, &host_tx, &floodsub).await?;
        tokio::time::sleep(CLI_DELAY).await;
    }

    Ok(())
}
