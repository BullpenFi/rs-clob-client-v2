//! Serde helpers for flexible deserialization.
//!
//! When the `tracing` feature is enabled, this module also logs unknown fields
//! and deserialization failure paths so API drift is easier to spot.

use serde::Deserialize as _;
use serde::de::DeserializeOwned;
use serde_json::Value;

#[cfg(feature = "clob")]
use crate::clob::types::TickSize;
use crate::types::Decimal;

#[cfg(feature = "tracing")]
pub fn deserialize_with_warnings<T: DeserializeOwned>(value: Value) -> crate::Result<T> {
    use std::any::type_name;

    tracing::trace!(
        type_name = %type_name::<T>(),
        json = %value,
        "deserializing JSON"
    );

    let original = value.clone();
    let mut unknown_paths: Vec<String> = Vec::new();

    let result: T = serde_ignored::deserialize(value, |path| {
        unknown_paths.push(path.to_string());
    })
    .inspect_err(|_| {
        let json_str = original.to_string();
        let deserializer = &mut serde_json::Deserializer::from_str(&json_str);
        let path_result: Result<T, _> = serde_path_to_error::deserialize(deserializer);
        if let Err(path_error) = path_result {
            let path = path_error.path().to_string();
            let inner_error = path_error.inner();
            let value_at_path = lookup_value(&original, &path);
            let value_display = format_value(value_at_path);

            tracing::error!(
                type_name = %type_name::<T>(),
                path = %path,
                value = %value_display,
                error = %inner_error,
                "deserialization failed"
            );
        }
    })?;

    if !unknown_paths.is_empty() {
        for path in unknown_paths {
            let field_value = lookup_value(&original, &path);
            let value_display = format_value(field_value);

            tracing::warn!(
                type_name = %type_name::<T>(),
                field = %path,
                value = %value_display,
                "unknown field in API response"
            );
        }
    }

    Ok(result)
}

#[cfg(not(feature = "tracing"))]
pub fn deserialize_with_warnings<T: DeserializeOwned>(value: Value) -> crate::Result<T> {
    Ok(serde_json::from_value(value)?)
}

#[cfg(feature = "clob")]
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
            )));
        }
    };

    raw.parse().map_err(serde::de::Error::custom)
}

pub fn deserialize_optional_decimal<'de, D>(
    deserializer: D,
) -> std::result::Result<Option<Decimal>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Option::<Value>::deserialize(deserializer)?;
    let Some(value) = value else {
        return Ok(None);
    };

    let raw = match value {
        Value::String(value) => value,
        Value::Number(value) => value.to_string(),
        other => {
            return Err(serde::de::Error::custom(format!(
                "expected decimal as string or number, got {other}"
            )));
        }
    };

    raw.parse::<Decimal>()
        .map(Some)
        .map_err(serde::de::Error::custom)
}

#[cfg(feature = "tracing")]
fn lookup_value<'value>(value: &'value Value, path: &str) -> Option<&'value Value> {
    if path.is_empty() {
        return Some(value);
    }

    let mut current = value;
    let segments = parse_path_segments(path);

    for segment in segments {
        if segment.is_empty() || segment == "?" {
            continue;
        }

        match current {
            Value::Object(map) => current = map.get(&segment)?,
            Value::Array(array) => {
                let index: usize = segment.parse().ok()?;
                current = array.get(index)?;
            }
            _ => return None,
        }
    }

    Some(current)
}

#[cfg(feature = "tracing")]
fn parse_path_segments(path: &str) -> Vec<String> {
    let mut segments = Vec::new();
    let mut current = String::new();
    let mut chars = path.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '.' => {
                if !current.is_empty() {
                    segments.push(std::mem::take(&mut current));
                }
            }
            '[' => {
                if !current.is_empty() {
                    segments.push(std::mem::take(&mut current));
                }
                for inner in chars.by_ref() {
                    if inner == ']' {
                        break;
                    }
                    current.push(inner);
                }
                if !current.is_empty() {
                    segments.push(std::mem::take(&mut current));
                }
            }
            ']' => {}
            _ => current.push(ch),
        }
    }

    if !current.is_empty() {
        segments.push(current);
    }

    segments
}

#[cfg(feature = "tracing")]
fn format_value(value: Option<&Value>) -> String {
    match value {
        Some(value) => value.to_string(),
        None => "<unable to retrieve>".to_owned(),
    }
}
