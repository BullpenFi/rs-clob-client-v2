use bon::Builder;
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
#[derive(Debug, Clone, Serialize, Deserialize, Builder, PartialEq, Eq)]
pub struct BuilderApiKey {
    pub key: String,
    pub secret: String,
    pub passphrase: String,
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
