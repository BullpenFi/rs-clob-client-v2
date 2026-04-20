#![allow(
    clippy::module_name_repetitions,
    reason = "Request suffix is intentional for API clarity"
)]

use bon::Builder;
use serde::Serialize;
use serde_with::{DisplayFromStr, serde_as, skip_serializing_none};

use crate::clob::types::{AssetType, PriceHistoryInterval, Side, SignatureTypeV2};
use crate::types::U256;

#[serde_as]
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Builder)]
#[builder(on(String, into))]
pub struct BookParams {
    #[serde_as(as = "DisplayFromStr")]
    pub token_id: U256,
    pub side: Side,
}

#[serde_as]
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Builder)]
#[builder(on(String, into))]
pub struct MidpointRequest {
    #[serde_as(as = "DisplayFromStr")]
    pub token_id: U256,
}

#[serde_as]
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Builder)]
#[builder(on(String, into))]
pub struct PriceRequest {
    #[serde_as(as = "DisplayFromStr")]
    pub token_id: U256,
    pub side: Side,
}

#[serde_as]
#[skip_serializing_none]
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Builder, Default)]
#[builder(on(String, into))]
pub struct SpreadRequest {
    #[serde_as(as = "DisplayFromStr")]
    pub token_id: U256,
    pub side: Option<Side>,
}

#[serde_as]
#[skip_serializing_none]
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Builder, Default)]
#[builder(on(String, into))]
pub struct OrderBookRequest {
    #[serde_as(as = "DisplayFromStr")]
    pub token_id: U256,
    pub side: Option<Side>,
}

#[serde_as]
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Builder)]
#[builder(on(String, into))]
pub struct LastTradePriceRequest {
    #[serde_as(as = "DisplayFromStr")]
    pub token_id: U256,
}

#[skip_serializing_none]
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Builder, Default)]
#[serde(rename_all = "camelCase")]
#[builder(on(String, into))]
pub struct PriceHistoryFilterParams {
    pub market: Option<String>,
    pub start_ts: Option<i64>,
    pub end_ts: Option<i64>,
    pub fidelity: Option<u32>,
    pub interval: Option<PriceHistoryInterval>,
}

#[skip_serializing_none]
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Builder, Default)]
#[builder(on(String, into))]
pub struct TradeParams {
    pub id: Option<String>,
    pub maker_address: Option<String>,
    pub market: Option<String>,
    pub asset_id: Option<String>,
    pub before: Option<String>,
    pub after: Option<String>,
}

#[skip_serializing_none]
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Builder, Default)]
#[builder(on(String, into))]
pub struct BuilderTradeParams {
    pub id: Option<String>,
    pub maker_address: Option<String>,
    pub market: Option<String>,
    pub asset_id: Option<String>,
    pub before: Option<String>,
    pub after: Option<String>,
    pub builder_code: Option<String>,
}

#[skip_serializing_none]
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Builder, Default)]
#[builder(on(String, into))]
pub struct OpenOrdersRequest {
    pub id: Option<String>,
    pub market: Option<String>,
    pub asset_id: Option<String>,
}

#[skip_serializing_none]
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Builder, Default)]
#[builder(on(String, into))]
pub struct DropNotificationsRequest {
    #[builder(default)]
    pub ids: Vec<String>,
}

#[skip_serializing_none]
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Builder)]
#[builder(on(String, into))]
pub struct BalanceAllowanceRequest {
    pub asset_type: AssetType,
    pub token_id: Option<String>,
    pub signature_type: Option<SignatureTypeV2>,
}

#[skip_serializing_none]
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Builder, Default)]
#[builder(on(String, into))]
pub struct OrderScoringRequest {
    pub order_id: Option<String>,
}

#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Builder, Default)]
#[builder(on(String, into))]
pub struct OrdersScoringRequest {
    #[serde(rename = "orderIds")]
    #[builder(default)]
    pub order_ids: Vec<String>,
}

#[skip_serializing_none]
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Builder, Default)]
#[builder(on(String, into))]
pub struct OrderMarketCancelRequest {
    pub market: Option<String>,
    pub asset_id: Option<String>,
}

#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Builder, Default)]
#[builder(on(String, into))]
pub struct UserRewardsEarningRequest {
    pub date: String,
    #[builder(default)]
    pub order_by: String,
    #[builder(default)]
    pub position: String,
    #[builder(default)]
    pub no_competition: bool,
}
