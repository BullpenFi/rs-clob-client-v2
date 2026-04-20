use bon::Builder;
use serde::{Deserialize, Serialize};

use crate::clob::types::Side;
use crate::types::Decimal;

#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize, Builder, PartialEq)]
pub struct MakerOrder {
    pub order_id: String,
    pub owner: String,
    pub maker_address: String,
    pub matched_amount: String,
    pub price: Decimal,
    pub fee_rate_bps: String,
    pub asset_id: String,
    pub outcome: String,
    pub side: Side,
}

#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize, Builder, PartialEq)]
pub struct Trade {
    pub id: String,
    pub taker_order_id: String,
    pub market: String,
    pub asset_id: String,
    pub side: Side,
    pub size: String,
    pub fee_rate_bps: String,
    pub price: Decimal,
    pub status: String,
    pub match_time: String,
    pub last_update: String,
    pub outcome: String,
    pub bucket_index: i64,
    pub owner: String,
    pub maker_address: String,
    #[builder(default)]
    #[serde(default)]
    pub maker_orders: Vec<MakerOrder>,
    pub transaction_hash: String,
    pub trader_side: String,
}

#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize, Builder, PartialEq)]
pub struct Notification {
    pub r#type: i32,
    pub owner: String,
    pub payload: serde_json::Value,
}

#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize, Builder, PartialEq)]
pub struct UserEarning {
    pub date: String,
    pub condition_id: String,
    pub asset_address: String,
    pub maker_address: String,
    pub earnings: Decimal,
    pub asset_rate: Decimal,
}

#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize, Builder, PartialEq)]
pub struct TotalUserEarning {
    pub date: String,
    pub asset_address: String,
    pub maker_address: String,
    pub earnings: Decimal,
    pub asset_rate: Decimal,
}

#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize, Builder, PartialEq)]
pub struct BuilderTrade {
    pub id: String,
    #[serde(rename = "tradeType")]
    pub trade_type: String,
    #[serde(rename = "takerOrderHash")]
    pub taker_order_hash: String,
    pub builder: String,
    pub market: String,
    #[serde(rename = "assetId")]
    pub asset_id: String,
    pub side: String,
    pub size: String,
    #[serde(rename = "sizeUsdc")]
    pub size_usdc: String,
    pub price: String,
    pub status: String,
    pub outcome: String,
    #[serde(rename = "outcomeIndex")]
    pub outcome_index: i64,
    pub owner: String,
    pub maker: String,
    #[serde(rename = "transactionHash")]
    pub transaction_hash: String,
    #[serde(rename = "matchTime")]
    pub match_time: String,
    #[serde(rename = "bucketIndex")]
    pub bucket_index: i64,
    pub fee: String,
    #[serde(rename = "feeUsdc")]
    pub fee_usdc: String,
    #[serde(default)]
    pub err_msg: Option<String>,
    #[serde(rename = "createdAt", default)]
    pub created_at: Option<String>,
    #[serde(rename = "updatedAt", default)]
    pub updated_at: Option<String>,
}

#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize, Builder, PartialEq)]
pub struct MarketTradeEventMarket {
    pub condition_id: String,
    pub asset_id: String,
    pub question: String,
    pub icon: String,
    pub slug: String,
}

#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize, Builder, PartialEq)]
pub struct MarketTradeEventUser {
    pub address: String,
    pub username: String,
    pub profile_picture: String,
    pub optimized_profile_picture: String,
    pub pseudonym: String,
}

#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize, Builder, PartialEq)]
pub struct MarketTradeEvent {
    pub event_type: String,
    pub market: MarketTradeEventMarket,
    pub user: MarketTradeEventUser,
    pub side: Side,
    pub size: String,
    pub fee_rate_bps: String,
    pub price: String,
    pub outcome: String,
    pub outcome_index: i64,
    pub transaction_hash: String,
    pub timestamp: String,
}
