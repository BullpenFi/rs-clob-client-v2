use bon::Builder;
use serde::Serialize;

use crate::ws::Subscription;

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
