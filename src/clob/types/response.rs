#![allow(
    clippy::module_name_repetitions,
    reason = "Response suffix is intentional for API clarity"
)]

use std::collections::HashMap;

use bon::Builder;
use serde::{Deserialize, Serialize};

use crate::auth::Credentials;
use crate::clob::types::{TickSize, trade::Trade, trade::BuilderTrade, trade::Notification};
use crate::serde_helpers::deserialize_tick_size;
use crate::types::Decimal;

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

#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize, Builder, PartialEq, Eq)]
pub struct BanStatus {
    pub closed_only: bool,
}

#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(bound(
    serialize = "T: Serialize",
    deserialize = "T: serde::Deserialize<'de>"
))]
pub struct Page<T> {
    pub limit: u32,
    pub count: u32,
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
#[derive(Debug, Clone, Serialize, Deserialize, Builder, PartialEq, Eq)]
pub struct FeeInfo {
    pub rate: u32,
    pub exponent: u32,
}

#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize, Builder, PartialEq)]
pub struct MarketPrice {
    pub t: i64,
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
    #[serde(default)]
    pub error_msg: Option<String>,
    pub order_id: String,
    #[builder(default)]
    #[serde(default)]
    pub transactions_hashes: Vec<String>,
    pub status: String,
    pub taking_amount: String,
    pub making_amount: String,
}

#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize, Builder, PartialEq, Eq)]
pub struct OrderScoringResponse {
    pub scoring: bool,
}

pub type OrdersScoringResponse = HashMap<String, bool>;
pub type RewardsPercentages = HashMap<String, Decimal>;

#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize, Builder, PartialEq)]
pub struct TradesPaginatedResponse {
    #[builder(default)]
    #[serde(default)]
    pub trades: Vec<Trade>,
    pub next_cursor: String,
    pub limit: u32,
    pub count: u32,
}

#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize, Builder, PartialEq)]
pub struct BuilderTradesResponse {
    #[builder(default)]
    #[serde(default)]
    pub trades: Vec<BuilderTrade>,
    pub next_cursor: String,
    pub limit: u32,
    pub count: u32,
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
