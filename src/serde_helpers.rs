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

    let redacted = redact_value(&value);

    tracing::trace!(
        type_name = %type_name::<T>(),
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
            let value_at_path = lookup_value(&redacted, &path);
            let value_display = format_value(value_at_path);

            tracing::error!(
                type_name = %type_name::<T>(),
                path = %path,
                value = %value_display,
                "deserialization failed"
            );
        }
    })?;

    if !unknown_paths.is_empty() {
        for path in unknown_paths {
            let field_value = lookup_value(&redacted, &path);
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

#[cfg(feature = "tracing")]
fn redact_value(value: &Value) -> Value {
    let mut redacted = value.clone();
    redact_value_in_place(&mut redacted);
    redacted
}

#[cfg(feature = "tracing")]
fn redact_value_in_place(value: &mut Value) {
    match value {
        Value::Object(map) => {
            for (key, nested) in map {
                if is_sensitive_field(key) {
                    *nested = Value::String("[REDACTED]".to_owned());
                } else {
                    redact_value_in_place(nested);
                }
            }
        }
        Value::Array(items) => {
            for item in items {
                redact_value_in_place(item);
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {}
    }
}

#[cfg(feature = "tracing")]
fn is_sensitive_field(field: &str) -> bool {
    let normalized = field
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .collect::<String>()
        .to_ascii_lowercase();

    matches!(
        normalized.as_str(),
        "apikey" | "authorization" | "passphrase" | "secret" | "token"
    )
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "tracing")]
    use std::sync::{Arc, Mutex};

    #[cfg(feature = "tracing")]
    use serde::Deserialize;
    #[cfg(feature = "tracing")]
    use serde_json::json;
    #[cfg(feature = "tracing")]
    use tracing::field::{Field, Visit};
    #[cfg(feature = "tracing")]
    use tracing::level_filters::LevelFilter;
    #[cfg(feature = "tracing")]
    use tracing::span::{Attributes, Id, Record};
    #[cfg(feature = "tracing")]
    use tracing::subscriber::{Interest, Subscriber};

    #[cfg(feature = "tracing")]
    use super::{deserialize_with_warnings, format_value, lookup_value, redact_value};

    #[cfg(feature = "tracing")]
    #[derive(Default)]
    struct RecordingSubscriber {
        events: Mutex<Vec<String>>,
    }

    #[cfg(feature = "tracing")]
    #[derive(Default)]
    struct EventVisitor {
        fields: Vec<String>,
    }

    #[cfg(feature = "tracing")]
    impl Visit for EventVisitor {
        fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
            self.fields.push(format!("{}={value:?}", field.name()));
        }

        fn record_str(&mut self, field: &Field, value: &str) {
            self.fields.push(format!("{}={value}", field.name()));
        }
    }

    #[cfg(feature = "tracing")]
    impl RecordingSubscriber {
        fn joined(&self) -> String {
            self.events.lock().expect("events lock").join("\n")
        }
    }

    #[cfg(feature = "tracing")]
    impl Subscriber for RecordingSubscriber {
        fn enabled(&self, _metadata: &tracing::Metadata<'_>) -> bool {
            true
        }

        fn new_span(&self, _span: &Attributes<'_>) -> Id {
            Id::from_u64(1)
        }

        fn record(&self, _span: &Id, _values: &Record<'_>) {}

        fn record_follows_from(&self, _span: &Id, _follows: &Id) {}

        fn event(&self, event: &tracing::Event<'_>) {
            let mut visitor = EventVisitor::default();
            event.record(&mut visitor);
            self.events
                .lock()
                .expect("events lock")
                .push(visitor.fields.join(" "));
        }

        fn enter(&self, _span: &Id) {}

        fn exit(&self, _span: &Id) {}

        fn register_callsite(&self, _metadata: &'static tracing::Metadata<'static>) -> Interest {
            Interest::always()
        }

        fn max_level_hint(&self) -> Option<LevelFilter> {
            Some(LevelFilter::TRACE)
        }

        fn clone_span(&self, id: &Id) -> Id {
            id.clone()
        }

        fn try_close(&self, _id: Id) -> bool {
            true
        }
    }

    #[cfg(feature = "tracing")]
    #[test]
    fn redact_value_redacts_nested_sensitive_fields() {
        let value = json!({
            "apiKey": "public-id",
            "nested": {
                "secret": "top-secret",
                "tokens": [
                    { "passphrase": "hunter2" },
                    { "keep": "visible" }
                ]
            }
        });

        let redacted = redact_value(&value);

        assert_eq!(redacted["apiKey"], "[REDACTED]");
        assert_eq!(redacted["nested"]["secret"], "[REDACTED]");
        assert_eq!(redacted["nested"]["tokens"][0]["passphrase"], "[REDACTED]");
        assert_eq!(redacted["nested"]["tokens"][1]["keep"], "visible");
    }

    #[cfg(feature = "tracing")]
    #[test]
    fn format_value_uses_redacted_payloads() {
        let value = json!({
            "credentials": {
                "passphrase": "secret-passphrase"
            }
        });
        let redacted = redact_value(&value);

        let formatted = format_value(lookup_value(&redacted, "credentials.passphrase"));

        assert_eq!(formatted, "\"[REDACTED]\"");
    }

    #[cfg(feature = "tracing")]
    #[test]
    fn deserialize_with_warnings_does_not_log_raw_sensitive_values_on_errors() {
        #[derive(Deserialize)]
        struct SensitivePayload {
            #[serde(rename = "secret")]
            _secret: u32,
        }

        let subscriber = Arc::new(RecordingSubscriber::default());
        let dispatch = tracing::Dispatch::new(Arc::clone(&subscriber));

        tracing::dispatcher::with_default(&dispatch, || {
            let result: crate::Result<SensitivePayload> =
                deserialize_with_warnings(json!({ "secret": "hunter2" }));
            assert!(result.is_err());
        });

        let logs = subscriber.joined();
        assert!(!logs.contains("hunter2"));
        assert!(logs.contains("[REDACTED]"));
    }

    #[cfg(feature = "tracing")]
    #[test]
    fn deserialize_with_warnings_redacts_unknown_sensitive_fields() {
        #[derive(Debug, Deserialize, PartialEq)]
        struct KnownPayload {
            keep: String,
        }

        let subscriber = Arc::new(RecordingSubscriber::default());
        let dispatch = tracing::Dispatch::new(Arc::clone(&subscriber));

        tracing::dispatcher::with_default(&dispatch, || {
            let result: crate::Result<KnownPayload> = deserialize_with_warnings(json!({
                "keep": "visible",
                "passphrase": "hunter2"
            }));
            assert_eq!(
                result.expect("payload"),
                KnownPayload {
                    keep: "visible".to_owned()
                }
            );
        });

        let logs = subscriber.joined();
        assert!(!logs.contains("hunter2"));
        assert!(logs.contains("[REDACTED]"));
    }
}
