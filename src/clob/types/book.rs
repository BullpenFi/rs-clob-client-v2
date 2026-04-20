use std::collections::HashMap;

use bon::Builder;
use serde::{Deserialize, Serialize};
use serde_with::{DisplayFromStr, serde_as};
// SHA-1 is used here for API compatibility with Polymarket's orderbook hash,
// not for cryptographic security. Do not replace with SHA-256.
use sha1::{Digest as _, Sha1};

use crate::Result;
use crate::clob::types::{Side, TickSize};
use crate::serde_helpers::deserialize_tick_size;
use crate::types::Decimal;

#[non_exhaustive]
#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize, Builder, PartialEq)]
pub struct OrderSummary {
    #[serde_as(as = "DisplayFromStr")]
    pub price: Decimal,
    #[serde_as(as = "DisplayFromStr")]
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

#[cfg(test)]
mod tests {
    use super::OrderSummary;
    use crate::types::Decimal;
    use std::str::FromStr as _;

    #[test]
    fn order_summary_serializes_decimal_fields_as_strings() {
        let summary = OrderSummary {
            price: Decimal::from_str("0.5").expect("decimal"),
            size: Decimal::from_str("100").expect("decimal"),
        };

        let json = serde_json::to_value(&summary).expect("order summary json");
        assert_eq!(json["price"], "0.5");
        assert_eq!(json["size"], "100");
    }
}
