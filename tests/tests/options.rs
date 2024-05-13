use pretty_assertions::assert_eq;
use tests::proto::options;

fn proto() -> options::Message {
    options::Message {
        address: "".to_string(),
        post_code: None,
        is_valid: false,
    }
}

#[test]
fn options() {
    const UNKNOWN_JSON: &str = r#"{"post_code":null,"is_valid":"FFFF","phone": "+1 10-1234-5678"}"#;
    let message = serde_json::from_str::<options::Message>(UNKNOWN_JSON).unwrap();
    assert_eq!(message, proto());
}
