mod testproto;

use crate::testproto::User;

fn main() {
    let u1 = r#"{
        "id": 1,
        "name": "John Doe",
        "email": "johndoe@example.com",
        "activation": "ACTIVATION_REQUESTED",
        "type": "USER_TYPE_REGULAR",
        "homepage": "https://www.johndoe.com"
    }"#;
    let u2 = r#"{
        "id": 2,
        "name": "Jane Doe",
        "email": "janedoe@example.com",
        "activation": "ACTIVATION_ACTIVATED",
        "type": "USER_TYPE_ADMIN"
    }"#;
    let u3 = r#"{
        "id": 2,
        "name": "Alan Smithee",
        "email": "alansmithee@example.com",
        "activation": "ACTIVATION_UNDEFINED",
        "type": "USER_TYPE_UNDEFINED"
    }"#;

    println!("{:#?}", serde_json::from_str::<User>(u1));
    println!("{:#?}", serde_json::from_str::<User>(u2));
    println!("{:#?}", serde_json::from_str::<User>(u3));
}
