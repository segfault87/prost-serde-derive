use pretty_assertions::assert_eq;
use tests::proto::enums;
use tests::serde_test;
use tests::util::{round_trip_from_json, round_trip_from_message};

const JSON: &str = r#"{"language":"LANGUAGE_ENGLISH","notification":"NOTIFICATION_EMAIL","sub_notification":null}"#;

fn proto() -> enums::Enum {
    enums::Enum {
        language: enums::Language::English as i32,
        notification: Some(enums::Notification::Email as i32),
        sub_notification: None,
    }
}

serde_test!(enums::Enum, JSON, proto());
