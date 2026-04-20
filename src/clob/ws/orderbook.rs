#![allow(
    clippy::module_name_repetitions,
    reason = "WebSocket orderbook names intentionally match the Polymarket channel naming"
)]

use bon::Builder;
use serde::Serialize;

use crate::ws::Subscription;

#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Builder)]
pub struct OrderbookSubscription {
    pub asset_ids: Vec<String>,
}

impl Subscription for OrderbookSubscription {
    fn channel(&self) -> &'static str {
        "market"
    }
}

pub type OrderbookMessage = serde_json::Value;
