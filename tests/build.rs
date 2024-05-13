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
        )
        .type_attribute(
            "optional.Optional",
            "#[derive(prost_serde_derive::Deserialize, prost_serde_derive::Serialize)]",
        )
        .type_attribute(
            "repeated.Repeated",
            "#[derive(prost_serde_derive::Deserialize, prost_serde_derive::Serialize)]",
        )
        .type_attribute(
            "enums.Enum",
            "#[derive(prost_serde_derive::Deserialize, prost_serde_derive::Serialize)]",
        )
        .type_attribute(
            "enums.Language",
            "#[derive(prost_serde_derive::Deserialize, prost_serde_derive::Serialize)]",
        )
        .type_attribute(
            "enums.Notification",
            "#[derive(prost_serde_derive::Deserialize, prost_serde_derive::Serialize)]",
        )
        .type_attribute(
            "message.Message",
            "#[derive(prost_serde_derive::Deserialize, prost_serde_derive::Serialize)]",
        )
        .type_attribute(
            "message.Address",
            "#[derive(prost_serde_derive::Deserialize, prost_serde_derive::Serialize)]",
        )
        .type_attribute(
            "message.PostCode",
            "#[derive(prost_serde_derive::Deserialize, prost_serde_derive::Serialize)]",
        )
        .type_attribute(
            "oneof.Oneof",
            "#[derive(prost_serde_derive::Deserialize, prost_serde_derive::Serialize)]",
        )
        .type_attribute(
            "oneof.Oneof.animal",
            "#[derive(prost_serde_derive::Deserialize, prost_serde_derive::Serialize)]",
        )
        .type_attribute(
            "oneof.Cat",
            "#[derive(prost_serde_derive::Deserialize, prost_serde_derive::Serialize)]",
        )
        .type_attribute(
            "oneof.Dog",
            "#[derive(prost_serde_derive::Deserialize, prost_serde_derive::Serialize)]",
        )
        .type_attribute(
            "oneof.Wolf",
            "#[derive(prost_serde_derive::Deserialize, prost_serde_derive::Serialize)]",
        )
        .type_attribute(
            "options.Message",
            "#[derive(prost_serde_derive::Deserialize, prost_serde_derive::Serialize)]",
        )
        .type_attribute(
            "options.Message",
            "#[prost_serde_derive(omit_type_errors, use_default_for_missing_fields, ignore_unknown_fields)]",
        );

    builder
        .compile_with_config(
            config,
            &[
                "proto/empty.proto",
                "proto/primitive.proto",
                "proto/optional.proto",
                "proto/repeated.proto",
                "proto/enums.proto",
                "proto/message.proto",
                "proto/oneof.proto",
                "proto/options.proto",
            ],
            &["proto"],
        )
        .unwrap();
}
