use pretty_assertions::assert_eq;
use tests::proto::primitive;
use tests::serde_test;
use tests::util::{round_trip_from_json, round_trip_from_message};

const JSON: &str = r#"{"id":39,"timestamp":100000000000000,"name":"name","hashed_password":"/+I/","is_active":true}"#;

fn proto() -> primitive::Primitive {
    primitive::Primitive {
        id: 39,
        timestamp: 100000000000000,
        name: "name".to_string(),
        hashed_password: [0xff, 0xe2, 0x3f].to_vec(),
        is_active: true,
    }
}

serde_test!(primitive::Primitive, JSON, proto());
