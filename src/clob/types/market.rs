#![allow(
    clippy::module_name_repetitions,
    reason = "Market-prefixed API models intentionally mirror Polymarket's schema names"
)]

use std::str::FromStr as _;

use bon::Builder;
use rust_decimal::prelude::ToPrimitive as _;
use serde::de::Error as _;
use serde::ser::Error as _;
use serde::{Deserialize, Serialize};

use crate::clob::types::TickSize;
use crate::serde_helpers::deserialize_optional_decimal;
use crate::types::Decimal;

fn deserialize_optional_u32_from_any<'de, D>(
    deserializer: D,
) -> std::result::Result<Option<u32>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Option::<serde_json::Value>::deserialize(deserializer)?;
    let Some(value) = value else {
        return Ok(None);
    };

    match value {
        serde_json::Value::String(value) => {
            value.parse::<u32>().map(Some).map_err(D::Error::custom)
        }
        serde_json::Value::Number(value) => value
            .as_u64()
            .and_then(|value| u32::try_from(value).ok())
            .ok_or_else(|| D::Error::custom("expected u32-compatible number"))
            .map(Some),
        other => Err(D::Error::custom(format!(
            "expected optional u32 as string or number, got {other}"
        ))),
    }
}

#[allow(
    clippy::exhaustive_structs,
    reason = "NumericTickSize is a fixed API-compatibility wrapper around Polymarket's shorthand market tick size"
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NumericTickSize(pub TickSize);

impl NumericTickSize {
    #[must_use]
    pub fn tick_size(self) -> TickSize {
        self.0
    }
}

impl From<NumericTickSize> for TickSize {
    fn from(value: NumericTickSize) -> Self {
        value.0
    }
}

impl From<TickSize> for NumericTickSize {
    fn from(value: TickSize) -> Self {
        Self(value)
    }
}

impl Serialize for NumericTickSize {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let numeric = match self.0 {
            TickSize::Tenth => 0.1_f64,
            TickSize::Hundredth => 0.01_f64,
            TickSize::Thousandth => 0.001_f64,
            TickSize::TenThousandth => 0.0001_f64,
        };

        serializer.serialize_f64(numeric)
    }
}

impl<'de> Deserialize<'de> for NumericTickSize {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;
        let raw = match value {
            serde_json::Value::String(value) => value,
            serde_json::Value::Number(value) => value.to_string(),
            other => {
                return Err(D::Error::custom(format!(
                    "expected tick size as string or number, got {other}"
                )));
            }
        };

        TickSize::from_str(&raw).map(Self).map_err(D::Error::custom)
    }
}

#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize, Builder, PartialEq, Eq)]
pub struct ClobToken {
    #[serde(rename = "t")]
    pub token_id: String,
    #[serde(rename = "o")]
    pub outcome: String,
}

#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize, Builder, PartialEq)]
pub struct FeeDetails {
    #[serde(
        rename = "r",
        deserialize_with = "deserialize_optional_decimal",
        serialize_with = "serialize_optional_decimal_as_number"
    )]
    pub rate: Option<Decimal>,
    #[serde(rename = "e")]
    pub exponent: Option<u32>,
    #[serde(rename = "to")]
    pub taker_only: bool,
}

#[allow(
    clippy::ref_option,
    reason = "serde serialize_with hooks for optional fields receive &Option<T>"
)]
fn serialize_optional_decimal_as_number<S>(
    value: &Option<Decimal>,
    serializer: S,
) -> std::result::Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    match value {
        Some(value) => serializer.serialize_some(
            &value
                .to_f64()
                .ok_or_else(|| S::Error::custom("decimal cannot be represented as f64"))?,
        ),
        None => serializer.serialize_none(),
    }
}

#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize, Builder, PartialEq)]
pub struct MarketDetails {
    #[serde(rename = "c")]
    pub condition_id: String,
    #[builder(default)]
    #[serde(default)]
    pub t: [Option<ClobToken>; 2],
    #[serde(rename = "mts")]
    pub minimum_tick_size: NumericTickSize,
    #[serde(rename = "nr")]
    pub neg_risk: bool,
    #[serde(rename = "fd")]
    pub fee_details: Option<FeeDetails>,
    #[serde(
        rename = "mbf",
        default,
        deserialize_with = "deserialize_optional_u32_from_any"
    )]
    pub maker_base_fee: Option<u32>,
    #[serde(
        rename = "tbf",
        default,
        deserialize_with = "deserialize_optional_u32_from_any"
    )]
    pub taker_base_fee: Option<u32>,
}

#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize, Builder, PartialEq)]
pub struct Token {
    pub token_id: String,
    pub outcome: String,
    pub price: Decimal,
    #[serde(default)]
    pub winner: bool,
}

#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize, Builder, PartialEq)]
pub struct RewardsConfig {
    pub asset_address: String,
    pub start_date: String,
    pub end_date: String,
    pub rate_per_day: Decimal,
    pub total_rewards: Decimal,
}

#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize, Builder, PartialEq)]
pub struct MarketReward {
    pub condition_id: String,
    pub question: String,
    pub market_slug: String,
    pub event_slug: String,
    pub image: String,
    pub rewards_max_spread: Decimal,
    pub rewards_min_size: Decimal,
    #[builder(default)]
    #[serde(default)]
    pub tokens: Vec<Token>,
    #[builder(default)]
    #[serde(default)]
    pub rewards_config: Vec<RewardsConfig>,
}

#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize, Builder, PartialEq)]
pub struct Earning {
    pub asset_address: String,
    pub earnings: Decimal,
    pub asset_rate: Decimal,
}

#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize, Builder, PartialEq)]
pub struct UserRewardsEarning {
    pub condition_id: String,
    pub question: String,
    pub market_slug: String,
    pub event_slug: String,
    pub image: String,
    pub rewards_max_spread: Decimal,
    pub rewards_min_size: Decimal,
    pub market_competitiveness: Decimal,
    #[builder(default)]
    #[serde(default)]
    pub tokens: Vec<Token>,
    #[builder(default)]
    #[serde(default)]
    pub rewards_config: Vec<RewardsConfig>,
    pub maker_address: String,
    pub earning_percentage: Decimal,
    #[builder(default)]
    #[serde(default)]
    pub earnings: Vec<Earning>,
}

#[non_exhaustive]
#[allow(
    clippy::struct_excessive_bools,
    reason = "The market payload is a direct mirror of Polymarket's boolean-heavy API response"
)]
#[derive(Debug, Clone, Serialize, Deserialize, Builder, PartialEq)]
pub struct Market {
    pub enable_order_book: bool,
    pub active: bool,
    pub closed: bool,
    pub archived: bool,
    pub accepting_orders: bool,
    pub accepting_order_timestamp: Option<String>,
    pub minimum_order_size: Decimal,
    pub minimum_tick_size: Decimal,
    pub condition_id: Option<String>,
    pub question_id: Option<String>,
    pub question: String,
    pub description: String,
    pub market_slug: String,
    pub end_date_iso: Option<String>,
    pub game_start_time: Option<String>,
    pub seconds_delay: u64,
    pub fpmm: Option<String>,
    pub maker_base_fee: Decimal,
    pub taker_base_fee: Decimal,
    pub notifications_enabled: bool,
    pub neg_risk: bool,
    pub neg_risk_market_id: Option<String>,
    pub neg_risk_request_id: Option<String>,
    pub icon: String,
    pub image: String,
    #[serde(default)]
    pub tokens: Vec<Token>,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize, Builder, PartialEq)]
pub struct SimplifiedMarket {
    pub condition_id: Option<String>,
    #[serde(default)]
    pub tokens: Vec<Token>,
}
