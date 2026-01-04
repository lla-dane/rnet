pub mod floodsub {
    include!(concat!(env!("OUT_DIR"), "/floodsub.pb.rs"));
}

pub mod gossipsub {
    include!(concat!(env!("OUT_DIR"), "/gossipsub.pb.rs"));
}