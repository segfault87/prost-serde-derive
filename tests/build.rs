fn main() {
    let mut config = prost_build::Config::new();
    config.bytes(["testproto.User.api_keys"]);

    let builder = tonic_build::configure()
        .out_dir("src/proto")
        .type_attribute(
            "empty.Empty",
            "#[derive(prost_serde_derive::Deserialize, prost_serde_derive::Serialize)]",
        )
        .type_attribute(
            "primitive.Primitive",
            "#[derive(prost_serde_derive::Deserialize, prost_serde_derive::Serialize)]",
        );

    builder
        .compile_with_config(
            config,
            &["proto/empty.proto", "proto/primitive.proto"],
            &["proto"],
        )
        .unwrap();
}
