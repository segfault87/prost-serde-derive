use pretty_assertions::assert_eq;
use tests::proto::empty;
use tests::serde_test;
use tests::util::{round_trip_from_json, round_trip_from_message};

const JSON: &str = r#"{}"#;

fn proto() -> empty::Empty {
    empty::Empty {}
}

serde_test!(empty::Empty, JSON, proto());
