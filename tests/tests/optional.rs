use pretty_assertions::assert_eq;
use tests::proto::optional;
use tests::serde_test;

const JSON: &str = r#"{"id":39,"timestamp":100000000000000,"name":"name","hashed_password":null,"is_active":null}"#;

fn proto() -> optional::Optional {
    optional::Optional {
        id: Some(39),
        timestamp: Some(100000000000000),
        name: Some("name".to_string()),
        ..Default::default()
    }
}

serde_test!(optional::Optional, JSON, proto());
