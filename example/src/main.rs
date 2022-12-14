mod testproto;

use crate::testproto::User;

fn main() {
    let u1 = r#"{
        "id": 1,
        "name": "John Doe",
        "email": "johndoe@example.com",
        "hashed_password": "aGVsbG8=",
        "activation": "ACTIVATION_REQUESTED",
        "type": "USER_TYPE_REGULAR",
        "homepage": "https://www.johndoe.com",
        "api_keys": ["YQ==", "Yg=="]
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

    println!("{:#?}", serde_json::from_str::<User>(u1));
    println!("{:#?}", serde_json::from_str::<User>(u2));
    println!("{:#?}", serde_json::from_str::<User>(u3));
}
