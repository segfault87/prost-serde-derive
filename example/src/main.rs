mod testproto;

use crate::testproto::{Activation, User};

fn main() {
    let u1 = r#"{
        "id": 1,
        "name": "John Doe",
        "email": "johndoe@example.com",
        "hashed_password": "aGVsbG8=",
        "activation": "ACTIVATION_REQUESTED",
        "type": "USER_TYPE_REGULAR",
        "homepage": "https://www.johndoe.com",
        "api_keys": ["YQ==", "Yg=="],
        "paid_type": "PAID_TYPE_MONTHLY"
    }"#;
    let u2 = r#"{
        "id": 2,
        "name": "Jane Doe",
        "email": "janedoe@example.com",
        "hashed_password": "",
        "activation": "ACTIVATION_ACTIVATED",
        "type": "USER_TYPE_ADMIN",
        "api_keys": [],
        "permissions": ["USER_PERMISSION_READ_POSTS", "USER_PERMISSION_WRITE_POSTS"]
    }"#;
    let u3 = r#"{
        "name": "Alan Smithee",
        "email": "alansmithee@example.com",
        "hashed_password": 5,
        "activation": "ACTIVATION_REVOKED",
        "permissions": ["USER_PERMISSION_UPDATE_POSTS"]
    }"#;

    let d1 = serde_json::from_str::<User>(u1).unwrap();
    let d2 = serde_json::from_str::<User>(u2).unwrap();
    let d3 = serde_json::from_str::<User>(u3).unwrap();

    println!("{:#?}", d1);
    println!("{:#?}", d2);
    println!("{:#?}", d3);

    println!("{}", serde_json::to_string(&d1).unwrap());
    println!("{}", serde_json::to_string(&d2).unwrap());
    println!("{}", serde_json::to_string(&d3).unwrap());

    println!(
        " {:?}",
        serde_json::from_str::<Activation>(r#""ACTIVATION_ACTIVATED""#)
    );
    println!(
        "{:?}",
        serde_json::from_str::<Activation>(r#""ACTIVATION_NON_EXISTENT""#)
    );

    println!("{}", serde_json::to_string(&Activation::Revoked).unwrap())
}
