use serde::de::DeserializeOwned;
use serde::Serialize;

fn drop_null_in_object(object: &mut serde_json::Map<String, serde_json::Value>) {
    object.retain(|_, v| !v.is_null());

    for value in object.values_mut() {
        if let serde_json::Value::Object(object) = value {
            object.retain(|_, v| !v.is_null());
        }
    }
}

pub fn drop_null(json: &str) -> String {
    let mut value: serde_json::Value = serde_json::from_str(json).unwrap();
    if let serde_json::Value::Object(object) = &mut value {
        drop_null_in_object(object);
    }

    serde_json::to_string(&value).unwrap()
}

pub fn round_trip_from_json<T: Serialize + DeserializeOwned>(
    json: &str,
    is_drop_null: bool,
) -> String {
    let mut json = json.to_string();
    if is_drop_null {
        json = drop_null(&json);
    }

    let result =
        serde_json::to_string(&serde_json::from_reader::<_, T>(json.as_bytes()).unwrap()).unwrap();
    if is_drop_null {
        drop_null(&result)
    } else {
        result
    }
}

pub fn round_trip_from_message<T: Serialize + DeserializeOwned>(
    message: T,
    is_drop_null: bool,
) -> T {
    let mut json = serde_json::to_string(&message).unwrap();
    if is_drop_null {
        json = drop_null(&json);
    }

    serde_json::from_reader::<_, T>(json.as_bytes()).unwrap()
}

#[macro_export]
macro_rules! serde_test {
    ($ty:ty, $json:expr, $value:expr) => {
        #[test]
        fn serialize() {
            assert_eq!(serde_json::to_string(&$value).unwrap(), $json);
        }

        #[test]
        fn deserialize() {
            assert_eq!(serde_json::from_str::<$ty>($json).unwrap(), $value);
        }

        #[test]
        fn round_trip() {
            assert_eq!(round_trip_from_json::<$ty>($json, false), $json);

            let mesage = $value;
            assert_eq!(round_trip_from_message(mesage.clone(), false), mesage);
        }

        #[test]
        fn round_trip_drop_null() {
            assert_eq!(
                round_trip_from_json::<$ty>($json, true),
                tests::util::drop_null($json)
            );

            let mesage = $value;
            assert_eq!(round_trip_from_message(mesage.clone(), true), mesage);
        }
    };
}
