use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

pub fn round_trip_from_json<'de, T: Serialize + Deserialize<'de>>(json: &'de str) -> String {
    serde_json::to_string(&serde_json::from_str::<T>(json).unwrap()).unwrap()
}

pub fn round_trip_from_message<'de, T: Serialize + DeserializeOwned>(message: T) -> T {
    let json = serde_json::to_string(&message).unwrap();
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
            assert_eq!(round_trip_from_json::<$ty>($json), $json);

            let mesage = $value;
            assert_eq!(round_trip_from_message(mesage.clone()), mesage);
        }
    };
}
