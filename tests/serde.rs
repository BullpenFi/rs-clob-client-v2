use std::str::FromStr as _;

use polymarket_clob_client_v2::clob::types::{
    BuilderTradesResponse, FeeInfo, MarketDetails, MarketPrice, MarketReward, OpenOrder,
    OrderBookSummary, OrderType, Page, RewardsPercentages, Side, SignatureTypeV2, TickSize,
    Token, Trade, TradesPaginatedResponse, UserRewardsEarning,
};
use polymarket_clob_client_v2::types::Decimal;
use serde_json::Value;

fn dec(value: &str) -> Decimal {
    Decimal::from_str(value).expect("decimal")
}

#[test]
fn side_buy_serializes_as_string() {
    assert_eq!(serde_json::to_value(Side::Buy).expect("json"), "BUY");
}

#[test]
fn side_sell_serializes_as_string() {
    assert_eq!(serde_json::to_value(Side::Sell).expect("json"), "SELL");
}

#[test]
fn signature_type_serializes_as_number() {
    assert_eq!(serde_json::to_value(SignatureTypeV2::Eoa).expect("json"), 0);
}

#[test]
fn order_type_serializes_as_string() {
    assert_eq!(serde_json::to_value(OrderType::Gtc).expect("json"), "GTC");
}

#[test]
fn tick_size_round_trip() {
    for (tick_size, expected) in [
        (TickSize::Tenth, "0.1"),
        (TickSize::Hundredth, "0.01"),
        (TickSize::Thousandth, "0.001"),
        (TickSize::TenThousandth, "0.0001"),
    ] {
        let json = serde_json::to_string(&tick_size).expect("serialize tick size");
        assert_eq!(json, format!("\"{expected}\""));
        let round_trip: TickSize = serde_json::from_str(&json).expect("deserialize tick size");
        assert_eq!(round_trip, tick_size);
    }
}

#[test]
fn market_details_shorthand_fields() {
    let details: MarketDetails = serde_json::from_str(
        r#"{
            "c": "condition-1",
            "t": [{ "t": "123", "o": "YES" }, null],
            "mts": 0.01,
            "nr": false,
            "fd": { "r": 0.02, "e": 1, "to": true },
            "mbf": 1,
            "tbf": "2"
        }"#,
    )
    .expect("market details");

    assert_eq!(details.condition_id, "condition-1");
    assert_eq!(details.minimum_tick_size.tick_size(), TickSize::Hundredth);
    assert!(!details.neg_risk);
    assert_eq!(details.t[0].as_ref().expect("token").token_id, "123");
    assert!(details.t[1].is_none());
    assert_eq!(
        details.fee_details.as_ref().expect("fee details").rate,
        Some(dec("0.02"))
    );
    assert_eq!(details.maker_base_fee, Some(1));
    assert_eq!(details.taker_base_fee, Some(2));

    let value = serde_json::to_value(&details).expect("serialize market details");
    assert_eq!(
        value,
        serde_json::json!({
            "c": "condition-1",
            "t": [{ "t": "123", "o": "YES" }, null],
            "mts": 0.01,
            "nr": false,
            "fd": { "r": 0.02, "e": 1, "to": true },
            "mbf": 1,
            "tbf": 2
        })
    );
}

#[test]
fn open_order_all_fields() {
    let order: OpenOrder = serde_json::from_str(
        r#"{
            "id": "order-1",
            "status": "live",
            "owner": "00000000-0000-0000-0000-000000000000",
            "maker_address": "0xf39fd6e51aad88f6f4ce6ab8827279cfffb92266",
            "market": "market-1",
            "asset_id": "123",
            "side": "BUY",
            "original_size": "10",
            "size_matched": "2",
            "price": "0.45",
            "associate_trades": ["trade-1"],
            "outcome": "YES",
            "created_at": 1700000000,
            "expiration": "0",
            "order_type": "GTC"
        }"#,
    )
    .expect("open order");

    assert_eq!(order.id, "order-1");
    assert_eq!(order.status, "live");
    assert_eq!(order.original_size, "10");
    assert_eq!(order.size_matched, "2");
    assert_eq!(order.price, "0.45");
    assert_eq!(order.associate_trades, vec!["trade-1".to_owned()]);
    assert_eq!(order.created_at, 1_700_000_000);
}

#[test]
fn trade_all_fields() {
    let trade: Trade = serde_json::from_str(
        r#"{
            "id": "trade-1",
            "taker_order_id": "order-2",
            "market": "market-1",
            "asset_id": "123",
            "side": "SELL",
            "size": "25",
            "fee_rate_bps": "10",
            "price": "0.42",
            "status": "matched",
            "match_time": "1700000001",
            "last_update": "1700000002",
            "outcome": "NO",
            "bucket_index": 7,
            "owner": "00000000-0000-0000-0000-000000000000",
            "maker_address": "0xf39fd6e51aad88f6f4ce6ab8827279cfffb92266",
            "maker_orders": [{
                "order_id": "maker-1",
                "owner": "00000000-0000-0000-0000-000000000000",
                "maker_address": "0xf39fd6e51aad88f6f4ce6ab8827279cfffb92266",
                "matched_amount": "25",
                "price": "0.42",
                "fee_rate_bps": "10",
                "asset_id": "123",
                "outcome": "NO",
                "side": "SELL"
            }],
            "transaction_hash": "0xabc",
            "trader_side": "TAKER"
        }"#,
    )
    .expect("trade");

    assert_eq!(trade.id, "trade-1");
    assert_eq!(trade.side, Side::Sell);
    assert_eq!(trade.price, dec("0.42"));
    assert_eq!(trade.maker_orders.len(), 1);
    assert_eq!(trade.maker_orders[0].price, dec("0.42"));
    assert_eq!(trade.transaction_hash, "0xabc");
}

#[test]
fn order_book_summary_hash() {
    let book: OrderBookSummary = serde_json::from_str(
        r#"{
            "market": "market-1",
            "asset_id": "123",
            "timestamp": "1700000000",
            "bids": [{ "price": "0.45", "size": "100" }],
            "asks": [{ "price": "0.55", "size": "100" }],
            "min_order_size": "1",
            "tick_size": "0.01",
            "neg_risk": false,
            "hash": null
        }"#,
    )
    .expect("order book");

    let hash = book.hash().expect("hash");
    assert_eq!(hash.len(), 40);
    assert!(hash.chars().all(|character| character.is_ascii_hexdigit()));
}

#[test]
fn token_serializes_price_as_number_and_omits_winner() {
    let token: Token = serde_json::from_str(
        r#"{
            "token_id": "123",
            "outcome": "YES",
            "price": "0.52",
            "winner": true
        }"#,
    )
    .expect("token");

    assert_eq!(token.price, dec("0.52"));
    assert!(token.winner);

    let value = serde_json::to_value(&token).expect("serialize token");
    assert_eq!(value.get("winner"), None);
    assert_eq!(value.get("token_id").and_then(Value::as_str), Some("123"));
    assert_eq!(value.get("outcome").and_then(Value::as_str), Some("YES"));
    assert_eq!(value.get("price").and_then(Value::as_f64), Some(0.52));
}

#[test]
fn reward_models_serialize_ts_number_fields_as_numbers() {
    let reward: MarketReward = serde_json::from_str(
        r#"{
            "condition_id": "condition-1",
            "question": "Will it rain?",
            "market_slug": "will-it-rain",
            "event_slug": "weather",
            "image": "https://example.com/image.png",
            "rewards_max_spread": "0.03",
            "rewards_min_size": "25",
            "tokens": [{
                "token_id": "123",
                "outcome": "YES",
                "price": "0.52",
                "winner": false
            }],
            "rewards_config": [{
                "asset_address": "0xabc",
                "start_date": "2024-01-01",
                "end_date": "2024-01-02",
                "rate_per_day": "1.25",
                "total_rewards": "10"
            }]
        }"#,
    )
    .expect("market reward");

    let value = serde_json::to_value(&reward).expect("serialize market reward");
    assert_eq!(
        value.get("rewards_max_spread").and_then(Value::as_f64),
        Some(0.03)
    );
    assert_eq!(
        value.get("rewards_min_size").and_then(Value::as_f64),
        Some(25.0)
    );
    assert_eq!(
        value["tokens"][0].get("price").and_then(Value::as_f64),
        Some(0.52)
    );
    assert_eq!(value["tokens"][0].get("winner"), None);
    assert_eq!(
        value["rewards_config"][0]
            .get("rate_per_day")
            .and_then(Value::as_f64),
        Some(1.25)
    );
    assert_eq!(
        value["rewards_config"][0]
            .get("total_rewards")
            .and_then(Value::as_f64),
        Some(10.0)
    );
}

#[test]
fn user_rewards_earning_serializes_nested_ts_number_fields_as_numbers() {
    let earnings: UserRewardsEarning = serde_json::from_str(
        r#"{
            "condition_id": "condition-1",
            "question": "Will it rain?",
            "market_slug": "will-it-rain",
            "event_slug": "weather",
            "image": "https://example.com/image.png",
            "rewards_max_spread": "0.03",
            "rewards_min_size": "25",
            "market_competitiveness": "0.8",
            "tokens": [{
                "token_id": "123",
                "outcome": "YES",
                "price": "0.52"
            }],
            "rewards_config": [{
                "asset_address": "0xabc",
                "start_date": "2024-01-01",
                "end_date": "2024-01-02",
                "rate_per_day": "1.25",
                "total_rewards": "10"
            }],
            "maker_address": "0xmaker",
            "earning_percentage": "12.5",
            "earnings": [{
                "asset_address": "0xabc",
                "earnings": "3.5",
                "asset_rate": "0.25"
            }]
        }"#,
    )
    .expect("user rewards earning");

    let value = serde_json::to_value(&earnings).expect("serialize user rewards earning");
    assert_eq!(
        value.get("market_competitiveness").and_then(Value::as_f64),
        Some(0.8)
    );
    assert_eq!(
        value.get("earning_percentage").and_then(Value::as_f64),
        Some(12.5)
    );
    assert_eq!(
        value["earnings"][0].get("earnings").and_then(Value::as_f64),
        Some(3.5)
    );
    assert_eq!(
        value["earnings"][0]
            .get("asset_rate")
            .and_then(Value::as_f64),
        Some(0.25)
    );
}

#[test]
fn response_number_models_serialize_as_numbers() {
    let fee_info: FeeInfo =
        serde_json::from_str(r#"{ "rate": "0.02", "exponent": 2 }"#).expect("fee info");
    let market_price: MarketPrice =
        serde_json::from_str(r#"{ "t": 1700000000, "p": "0.42" }"#).expect("market price");

    let fee_value = serde_json::to_value(&fee_info).expect("serialize fee info");
    let price_value = serde_json::to_value(&market_price).expect("serialize market price");

    assert_eq!(fee_value.get("rate").and_then(Value::as_f64), Some(0.02));
    assert_eq!(price_value.get("p").and_then(Value::as_f64), Some(0.42));
}

#[test]
fn rewards_percentages_round_trip_as_numeric_map() {
    let percentages: RewardsPercentages = serde_json::from_str(
        r#"{
            "market-1": 12.5,
            "market-2": "7.25"
        }"#,
    )
    .expect("rewards percentages");

    assert_eq!(percentages.get("market-1"), Some(&dec("12.5")));
    assert_eq!(percentages.get("market-2"), Some(&dec("7.25")));

    let value = serde_json::to_value(&percentages).expect("serialize rewards percentages");
    assert_eq!(value.get("market-1").and_then(Value::as_f64), Some(12.5));
    assert_eq!(value.get("market-2").and_then(Value::as_f64), Some(7.25));
}

#[test]
fn pagination_models_allow_u64_limit_and_count() {
    let page: Page<Value> = serde_json::from_str(
        r#"{
            "limit": 4294967296,
            "count": 4294967297,
            "next_cursor": "LTE=",
            "data": []
        }"#,
    )
    .expect("page");

    assert_eq!(page.limit, 4_294_967_296);
    assert_eq!(page.count, 4_294_967_297);

    let trades: TradesPaginatedResponse = serde_json::from_str(
        r#"{
            "trades": [],
            "next_cursor": "LTE=",
            "limit": 4294967296,
            "count": 4294967297
        }"#,
    )
    .expect("trades paginated response");
    let builder_trades: BuilderTradesResponse = serde_json::from_str(
        r#"{
            "trades": [],
            "next_cursor": "LTE=",
            "limit": 4294967296,
            "count": 4294967297
        }"#,
    )
    .expect("builder trades response");

    assert_eq!(trades.limit, 4_294_967_296);
    assert_eq!(trades.count, 4_294_967_297);
    assert_eq!(builder_trades.limit, 4_294_967_296);
    assert_eq!(builder_trades.count, 4_294_967_297);
}
