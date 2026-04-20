use bon::Builder;
use serde::Serialize;

use crate::ws::Subscription;

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
