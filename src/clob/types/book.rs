use std::collections::HashMap;

use bon::Builder;
use serde::{Deserialize, Serialize};
use sha1::{Digest as _, Sha1};

use crate::clob::types::{Side, TickSize};
use crate::serde_helpers::deserialize_tick_size;
use crate::types::Decimal;
use crate::Result;

#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize, Builder, PartialEq)]
pub struct OrderSummary {
    pub price: Decimal,
    pub size: Decimal,
}

#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize, Builder, PartialEq)]
pub struct OrderBookSummary {
    pub market: String,
    pub asset_id: String,
    pub timestamp: String,
    #[builder(default)]
    #[serde(default)]
    pub bids: Vec<OrderSummary>,
    #[builder(default)]
    #[serde(default)]
    pub asks: Vec<OrderSummary>,
    pub min_order_size: String,
    #[serde(deserialize_with = "deserialize_tick_size")]
    pub tick_size: TickSize,
    pub neg_risk: bool,
    #[serde(default)]
    pub hash: Option<String>,
}

#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize, Builder, PartialEq)]
pub struct MidpointResponse {
    pub mid: Decimal,
}

#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize, Builder, PartialEq)]
#[serde(transparent)]
pub struct MidpointsResponse {
    pub midpoints: HashMap<String, Decimal>,
}

#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize, Builder, PartialEq)]
pub struct PriceResponse {
    pub price: Decimal,
}

#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize, Builder, PartialEq)]
#[serde(transparent)]
pub struct PricesResponse {
    pub prices: HashMap<String, HashMap<Side, Decimal>>,
}

#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize, Builder, PartialEq)]
pub struct SpreadResponse {
    pub spread: Decimal,
}

#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize, Builder, PartialEq)]
#[serde(transparent)]
pub struct SpreadsResponse {
    pub spreads: HashMap<String, Decimal>,
}

#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize, Builder, PartialEq)]
pub struct LastTradePriceResponse {
    pub price: Decimal,
    pub side: crate::clob::types::Side,
}

#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize, Builder, PartialEq)]
pub struct LastTradesPricesResponse {
    pub token_id: String,
    pub price: Decimal,
    pub side: crate::clob::types::Side,
}

impl OrderBookSummary {
    pub fn hash(&self) -> Result<String> {
        let mut order_book = self.clone();
        order_book.hash = Some(String::new());
        let json = serde_json::to_string(&order_book)?;
        let mut hasher = Sha1::new();
        hasher.update(json.as_bytes());
        Ok(format!("{:x}", hasher.finalize()))
    }
}
