use pretty_assertions::assert_eq;
use tests::proto::oneof;
use tests::serde_test;

const JSON: &str = r#"{"is_wild":true,"age":null,"cat":{"name":"name","color":"color"}}"#;

fn proto() -> oneof::Oneof {
    oneof::Oneof {
        animal: Some(oneof::oneof::Animal::Cat(oneof::Cat {
            name: "name".to_string(),
            color: "color".to_string(),
        })),
        is_wild: Some(true),
        age: None,
    }
}

serde_test!(oneof::Oneof, JSON, proto());
