fn main() {
    let mut config = prost_build::Config::new();
    config
        .compile_protos(
            &["proto/floodsub.proto", "proto/gossipsub.proto"], // All your files
            &["proto/"],                                    // Include paths
        )
        .unwrap();
}
