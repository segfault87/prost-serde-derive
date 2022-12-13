fn main() {
    tonic_build::configure()
        .out_dir("src")
        .type_attribute(
            "testproto.User",
            "#[derive(prost_serde_derive::Deserialize)]",
        )
        .field_attribute("testproto.User.activation", "#[enumeration(Activation)]")
        .field_attribute("testproto.User.type", "#[enumeration(UserType)]")
        .compile(&["proto/test.proto"], &["proto"])
        .unwrap();
}
