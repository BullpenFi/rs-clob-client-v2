use std::str::FromStr as _;

use polymarket_clob_client_v2::clob::types::{
    MarketDetails, OpenOrder, OrderBookSummary, OrderType, Side, SignatureTypeV2, TickSize, Trade,
};
use polymarket_clob_client_v2::types::Decimal;

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
            "fd": { "r": "0.02", "e": 1, "to": true },
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
            "fd": { "r": "0.02", "e": 1, "to": true },
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
