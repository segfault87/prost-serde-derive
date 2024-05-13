use pretty_assertions::assert_eq;
use tests::proto::message;
use tests::serde_test;

const JSON: &str = r#"{"address":{"street":null,"city":"city","state":"state"},"post_code":null}"#;

fn proto() -> message::Message {
    message::Message {
        address: Some(message::Address {
            street: None,
            city: Some("city".to_string()),
            state: "state".to_string(),
        }),
        post_code: None,
    }
}

serde_test!(message::Message, JSON, proto());
