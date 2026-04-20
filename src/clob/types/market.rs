use bon::Builder;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

use crate::clob::types::TickSize;
use crate::serde_helpers::{StringFromAny, deserialize_tick_size};
use crate::types::Decimal;

#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize, Builder, PartialEq, Eq)]
pub struct ClobToken {
    #[serde(rename = "t")]
    pub token_id: String,
    #[serde(rename = "o")]
    pub outcome: String,
}

#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize, Builder, PartialEq, Eq)]
pub struct FeeDetails {
    #[serde(rename = "r")]
    pub rate: Option<u32>,
    #[serde(rename = "e")]
    pub exponent: Option<u32>,
    #[serde(rename = "to")]
    pub taker_only: bool,
}

#[non_exhaustive]
#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize, Builder, PartialEq)]
pub struct MarketDetails {
    #[serde(rename = "c")]
    pub condition_id: String,
    #[builder(default)]
    #[serde(default)]
    pub t: Vec<Option<ClobToken>>,
    #[serde(rename = "mts", deserialize_with = "deserialize_tick_size")]
    pub minimum_tick_size: TickSize,
    #[serde(rename = "nr")]
    pub neg_risk: bool,
    #[serde(rename = "fd")]
    pub fee_details: Option<FeeDetails>,
    #[serde_as(as = "Option<StringFromAny>")]
    #[serde(rename = "mbf", default)]
    pub maker_base_fee: Option<String>,
    #[serde_as(as = "Option<StringFromAny>")]
    #[serde(rename = "tbf", default)]
    pub taker_base_fee: Option<String>,
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
