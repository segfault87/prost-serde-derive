use pretty_assertions::assert_eq;
use tests::proto::repeated;
use tests::serde_test;
use tests::util::{round_trip_from_json, round_trip_from_message};

const JSON: &str = r#"{"id":39,"timestamp":100000000000000,"names":["name","is","not","a","name"],"hashed_password":[],"is_active":true}"#;

fn proto() -> repeated::Repeated {
    repeated::Repeated {
        id: 39,
        timestamp: 100000000000000,
        names: vec![
            "name".to_string(),
            "is".to_string(),
            "not".to_string(),
            "a".to_string(),
            "name".to_string(),
        ],
        hashed_password: vec![],
        is_active: Some(true),
    }
}

serde_test!(repeated::Repeated, JSON, proto());
