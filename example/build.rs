fn main() {
    let mut config = prost_build::Config::new();
    config.bytes(["testproto.User.api_keys"]);

    let mut builder = tonic_build::configure()
        .out_dir("src")
        .type_attribute(
            "testproto.User",
            "#[derive(prost_serde_derive::Deserialize, prost_serde_derive::Serialize)]",
        )
        .type_attribute(
            "testproto.User",
            "#[prost_serde_derive(omit_type_errors, use_default_for_missing_fields)]",
        );

    for r#enum in &[
        "testproto.Activation",
        "testproto.UserType",
        "testproto.UserPermission",
    ] {
        builder = builder.type_attribute(
            r#enum,
            "#[derive(prost_serde_derive::Deserialize, prost_serde_derive::Serialize)]",
        )
    }

    builder
        .compile_with_config(config, &["proto/test.proto"], &["proto"])
        .unwrap();
}
