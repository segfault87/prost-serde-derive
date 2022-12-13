fn main() {
    tonic_build::configure()
        .out_dir("src")
        .type_attribute(
            "testproto.User",
            "#[derive(prost_serde_derive::Deserialize)]",
        )
        .compile(&["proto/test.proto"], &["proto"])
        .unwrap();
}
