use serde::Deserialize;
use serde::de::{self, DeserializeOwned, Visitor};
use serde_json::Value;

use crate::clob::types::TickSize;

pub struct StringFromAny;

impl<'de> serde_with::DeserializeAs<'de, String> for StringFromAny {
    fn deserialize_as<D>(deserializer: D) -> std::result::Result<String, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use std::fmt;

        struct StringOrNumberVisitor;

        impl Visitor<'_> for StringOrNumberVisitor {
            type Value = String;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("string or integer")
            }

            fn visit_str<E>(self, value: &str) -> std::result::Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(value.to_owned())
            }

            fn visit_string<E>(self, value: String) -> std::result::Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(value)
            }

            fn visit_i64<E>(self, value: i64) -> std::result::Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(value.to_string())
            }

            fn visit_u64<E>(self, value: u64) -> std::result::Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(value.to_string())
            }
        }

        deserializer.deserialize_any(StringOrNumberVisitor)
    }
}

impl serde_with::SerializeAs<String> for StringFromAny {
    fn serialize_as<S>(source: &String, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(source)
    }
}

pub fn deserialize_with_warnings<T: DeserializeOwned>(value: Value) -> crate::Result<T> {
    Ok(serde_json::from_value(value)?)
}

pub fn deserialize_tick_size<'de, D>(deserializer: D) -> std::result::Result<TickSize, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Value::deserialize(deserializer)?;
    let raw = match value {
        Value::String(value) => value,
        Value::Number(value) => value.to_string(),
        other => {
            return Err(serde::de::Error::custom(format!(
                "expected tick size as string or number, got {other}"
            )))
        }
    };

    raw.parse().map_err(serde::de::Error::custom)
}
