#![allow(
    clippy::module_name_repetitions,
    reason = "Builder-prefixed API models intentionally mirror Polymarket's endpoint names"
)]

use std::fmt;

use bon::Builder;
use secrecy::SecretString;
use serde::{Deserialize, Serialize};

#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize, Builder, PartialEq, Eq)]
pub struct BuilderConfig {
    #[serde(rename = "builderCode")]
    pub builder_code: String,
}

#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize, Builder, PartialEq)]
pub struct BuilderFeeRate {
    pub maker: f64,
    pub taker: f64,
}

#[non_exhaustive]
#[derive(Clone, Deserialize, Builder)]
pub struct BuilderApiKey {
    pub key: String,
    pub secret: SecretString,
    pub passphrase: SecretString,
}

impl fmt::Debug for BuilderApiKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BuilderApiKey")
            .field("key", &self.key)
            .field("secret", &"[REDACTED]")
            .field("passphrase", &"[REDACTED]")
            .finish()
    }
}

impl PartialEq for BuilderApiKey {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
    }
}

impl Eq for BuilderApiKey {
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_api_key_debug_redacts_secrets() {
        let key = BuilderApiKey {
            key: "builder-key".to_owned(),
            secret: SecretString::from("super-secret".to_owned()),
            passphrase: SecretString::from("very-secret".to_owned()),
        };

        let debug = format!("{key:?}");
        assert!(debug.contains("builder-key"));
        assert!(debug.contains("[REDACTED]"));
        assert!(!debug.contains("super-secret"));
        assert!(!debug.contains("very-secret"));
    }
}

#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize, Builder, PartialEq, Eq)]
pub struct BuilderApiKeyResponse {
    pub key: String,
    #[serde(rename = "createdAt")]
    pub created_at: Option<String>,
    #[serde(rename = "revokedAt")]
    pub revoked_at: Option<String>,
}

#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize, Builder, PartialEq, Eq)]
pub struct ReadonlyApiKeyResponse {
    #[serde(rename = "apiKey")]
    pub api_key: String,
}
