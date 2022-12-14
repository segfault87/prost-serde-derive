fn main() {
    let mut config = prost_build::Config::new();
    config.bytes(["testproto.User.api_keys"]);

    tonic_build::configure()
        .out_dir("src")
        .type_attribute(
            "testproto.User",
            "#[derive(prost_serde_derive::Deserialize)]\n#[prost_serde_derive(omit_type_errors, use_default_for_missing_fields)]",
        )
        .compile_with_config(config, &["proto/test.proto"], &["proto"])
        .unwrap();
}
