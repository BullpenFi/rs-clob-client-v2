#![allow(
    clippy::module_name_repetitions,
    reason = "WebSocket user-channel names intentionally match the Polymarket channel naming"
)]

use bon::Builder;
use serde::Serialize;

use crate::ws::Subscription;

#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Builder)]
pub struct UserSubscription {
    pub auth: String,
}

impl Subscription for UserSubscription {
    fn channel(&self) -> &'static str {
        "user"
    }
}

pub type UserMessage = serde_json::Value;
