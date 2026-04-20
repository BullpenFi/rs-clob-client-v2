#![allow(
    clippy::module_name_repetitions,
    reason = "Response suffix is intentional for API clarity"
)]

use std::collections::HashMap;
use std::ops::{Deref, DerefMut};

use bon::Builder;
use rust_decimal::prelude::ToPrimitive as _;
use secrecy::ExposeSecret as _;
use serde::de::Error as _;
use serde::ser::Error as _;
use serde::{Deserialize, Serialize};

use crate::auth::Credentials;
use crate::clob::types::{TickSize, trade::BuilderTrade, trade::Notification, trade::Trade};
use crate::serde_helpers::deserialize_tick_size;
use crate::types::Decimal;

fn deserialize_decimal_from_any<'de, D>(deserializer: D) -> std::result::Result<Decimal, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    let raw = match value {
        serde_json::Value::String(value) => value,
        serde_json::Value::Number(value) => value.to_string(),
        other => {
            return Err(D::Error::custom(format!(
                "expected decimal as string or number, got {other}"
            )));
        }
    };

    raw.parse::<Decimal>().map_err(D::Error::custom)
}

fn serialize_decimal_as_number<S>(
    value: &Decimal,
    serializer: S,
) -> std::result::Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_f64(
        value
            .to_f64()
            .ok_or_else(|| S::Error::custom("decimal cannot be represented as f64"))?,
    )
}

#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize, Builder, PartialEq)]
pub struct VersionResponse {
    pub version: Option<u32>,
}

#[non_exhaustive]
#[derive(Debug, Clone, Deserialize, Builder)]
pub struct ApiKeysResponse {
    #[serde(rename = "apiKeys")]
    #[builder(default)]
    #[serde(default)]
    pub api_keys: Vec<Credentials>,
}

impl Serialize for ApiKeysResponse {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        #[derive(Serialize)]
        struct SerializableCredentials<'credentials> {
            #[serde(rename = "apiKey")]
            key: crate::auth::ApiKey,
            secret: &'credentials str,
            passphrase: &'credentials str,
        }

        #[derive(Serialize)]
        struct SerializableApiKeysResponse<'credentials> {
            #[serde(rename = "apiKeys")]
            api_keys: Vec<SerializableCredentials<'credentials>>,
        }

        SerializableApiKeysResponse {
            api_keys: self
                .api_keys
                .iter()
                .map(|credentials| SerializableCredentials {
                    key: credentials.key(),
                    secret: credentials.secret().expose_secret(),
                    passphrase: credentials.passphrase().expose_secret(),
                })
                .collect(),
        }
        .serialize(serializer)
    }
}

#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize, Builder, PartialEq, Eq)]
pub struct BanStatus {
    pub closed_only: bool,
}

#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(bound(serialize = "T: Serialize", deserialize = "T: serde::Deserialize<'de>"))]
pub struct Page<T> {
    pub limit: u64,
    pub count: u64,
    pub next_cursor: String,
    #[serde(default)]
    pub data: Vec<T>,
}

#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize, Builder, PartialEq, Eq)]
pub struct TickSizeResponse {
    #[serde(deserialize_with = "deserialize_tick_size")]
    pub minimum_tick_size: TickSize,
}

#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize, Builder, PartialEq, Eq)]
pub struct NegRiskResponse {
    pub neg_risk: bool,
}

#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize, Builder, PartialEq, Eq)]
pub struct FeeRateResponse {
    pub base_fee: u32,
}

#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize, Builder, PartialEq)]
pub struct FeeInfo {
    #[serde(
        deserialize_with = "deserialize_decimal_from_any",
        serialize_with = "serialize_decimal_as_number"
    )]
    pub rate: Decimal,
    pub exponent: u32,
}

#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize, Builder, PartialEq)]
pub struct MarketPrice {
    pub t: i64,
    #[serde(
        deserialize_with = "deserialize_decimal_from_any",
        serialize_with = "serialize_decimal_as_number"
    )]
    pub p: Decimal,
}

#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize, Builder, PartialEq, Eq)]
pub struct BalanceAllowanceResponse {
    pub balance: String,
    pub allowance: String,
}

#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize, Builder, PartialEq, Eq)]
pub struct HeartbeatResponse {
    pub heartbeat_id: String,
    #[serde(default)]
    pub error_msg: Option<String>,
}

#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize, Builder, PartialEq, Eq)]
pub struct OpenOrder {
    pub id: String,
    pub status: String,
    pub owner: String,
    pub maker_address: String,
    pub market: String,
    pub asset_id: String,
    pub side: String,
    pub original_size: String,
    pub size_matched: String,
    pub price: String,
    #[builder(default)]
    #[serde(default)]
    pub associate_trades: Vec<String>,
    pub outcome: String,
    pub created_at: i64,
    pub expiration: String,
    pub order_type: String,
}

#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize, Builder, PartialEq, Eq)]
pub struct OrderResponse {
    pub success: bool,
    #[serde(rename = "errorMsg")]
    #[serde(default)]
    pub error_msg: Option<String>,
    #[serde(rename = "orderID")]
    pub order_id: String,
    #[serde(rename = "transactionsHashes")]
    #[builder(default)]
    #[serde(default)]
    pub transactions_hashes: Vec<String>,
    pub status: String,
    #[serde(rename = "takingAmount")]
    pub taking_amount: String,
    #[serde(rename = "makingAmount")]
    pub making_amount: String,
}

#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize, Builder, PartialEq, Eq)]
pub struct OrderScoringResponse {
    pub scoring: bool,
}

pub type OrdersScoringResponse = HashMap<String, bool>;

#[non_exhaustive]
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RewardsPercentages(HashMap<String, Decimal>);

impl RewardsPercentages {
    #[must_use]
    pub fn into_inner(self) -> HashMap<String, Decimal> {
        self.0
    }
}

impl From<HashMap<String, Decimal>> for RewardsPercentages {
    fn from(value: HashMap<String, Decimal>) -> Self {
        Self(value)
    }
}

impl Deref for RewardsPercentages {
    type Target = HashMap<String, Decimal>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for RewardsPercentages {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl IntoIterator for RewardsPercentages {
    type Item = (String, Decimal);
    type IntoIter = std::collections::hash_map::IntoIter<String, Decimal>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'percentages> IntoIterator for &'percentages RewardsPercentages {
    type Item = (&'percentages String, &'percentages Decimal);
    type IntoIter = std::collections::hash_map::Iter<'percentages, String, Decimal>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl<'percentages> IntoIterator for &'percentages mut RewardsPercentages {
    type Item = (&'percentages String, &'percentages mut Decimal);
    type IntoIter = std::collections::hash_map::IterMut<'percentages, String, Decimal>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter_mut()
    }
}

impl Serialize for RewardsPercentages {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap as _;

        let mut map = serializer.serialize_map(Some(self.0.len()))?;
        for (market, percentage) in &self.0 {
            let number = percentage
                .to_f64()
                .ok_or_else(|| S::Error::custom("decimal cannot be represented as f64"))?;
            map.serialize_entry(market, &number)?;
        }
        map.end()
    }
}

impl<'de> Deserialize<'de> for RewardsPercentages {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = HashMap::<String, serde_json::Value>::deserialize(deserializer)?;
        let mut parsed = HashMap::with_capacity(raw.len());

        for (market, value) in raw {
            let raw_decimal = match value {
                serde_json::Value::String(value) => value,
                serde_json::Value::Number(value) => value.to_string(),
                other => {
                    return Err(D::Error::custom(format!(
                        "expected decimal as string or number, got {other}"
                    )));
                }
            };

            let percentage = raw_decimal.parse::<Decimal>().map_err(D::Error::custom)?;
            parsed.insert(market, percentage);
        }

        Ok(Self(parsed))
    }
}

#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize, Builder, PartialEq)]
pub struct TradesPaginatedResponse {
    #[builder(default)]
    #[serde(default)]
    pub trades: Vec<Trade>,
    pub next_cursor: String,
    pub limit: u64,
    pub count: u64,
}

#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize, Builder, PartialEq)]
pub struct BuilderTradesResponse {
    #[builder(default)]
    #[serde(default)]
    pub trades: Vec<BuilderTrade>,
    pub next_cursor: String,
    pub limit: u64,
    pub count: u64,
}

#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize, Builder, PartialEq, Eq)]
pub struct BuilderFeesResponse {
    pub builder_maker_fee_rate_bps: u32,
    pub builder_taker_fee_rate_bps: u32,
}

#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize, Builder, PartialEq, Eq)]
pub struct ErrorResponse {
    pub error: serde_json::Value,
    pub status: Option<u16>,
}

pub type NotificationsResponse = Vec<Notification>;

#[cfg(test)]
mod tests {
    use super::OrderResponse;

    #[test]
    fn order_response_deserializes_api_camel_case_fields() {
        let response: OrderResponse = serde_json::from_str(
            r#"{
                "success": true,
                "errorMsg": "",
                "orderID": "abc",
                "transactionsHashes": [],
                "status": "live",
                "takingAmount": "100",
                "makingAmount": "50"
            }"#,
        )
        .expect("order response should deserialize");

        assert!(response.success);
        assert_eq!(response.error_msg.as_deref(), Some(""));
        assert_eq!(response.order_id, "abc");
        assert!(response.transactions_hashes.is_empty());
        assert_eq!(response.status, "live");
        assert_eq!(response.taking_amount, "100");
        assert_eq!(response.making_amount, "50");
    }
}
