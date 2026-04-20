use std::env;

use crate::docs::{client, server};

pub mod docs;

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    let mode = &args[1];

    if mode == "cl" {
        client().await.unwrap();
    } else {
        server().await.unwrap();
    }
}
