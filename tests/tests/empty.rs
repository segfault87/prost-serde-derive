use pretty_assertions::assert_eq;
use tests::proto::empty;
use tests::serde_test;

const JSON: &str = r#"{}"#;

fn proto() -> empty::Empty {
    empty::Empty {}
}

serde_test!(empty::Empty, JSON, proto());
