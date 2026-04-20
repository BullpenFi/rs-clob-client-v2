mod common;

use std::str::FromStr as _;

use alloy::primitives::U256;
use httpmock::Method::GET;
use httpmock::MockServer;
use polymarket_client_sdk::clob::types::{OrderType, Side};
use polymarket_client_sdk::clob::{Client, Config};
use polymarket_client_sdk::types::Decimal;

fn dec(value: &str) -> Decimal {
    Decimal::from_str(value).expect("decimal")
}

fn insecure_config() -> Config {
    Config::builder().allow_insecure(true).build()
}

fn order_summary(price: &str, size: &str) -> serde_json::Value {
    serde_json::json!({ "price": price, "size": size })
}

async fn calculate_market_price(
    token_id: U256,
    side: Side,
    amount: Decimal,
    order_type: OrderType,
    bids: Vec<serde_json::Value>,
    asks: Vec<serde_json::Value>,
) -> polymarket_client_sdk::Result<Decimal> {
    let server = MockServer::start();
    let token_id_string = token_id.to_string();
    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/book")
            .query_param("token_id", token_id_string.as_str());
        then.status(200).json_body_obj(&serde_json::json!({
            "market": "market-1",
            "asset_id": token_id_string,
            "timestamp": "1700000000",
            "bids": bids,
            "asks": asks,
            "min_order_size": "1",
            "tick_size": "0.01",
            "neg_risk": false,
            "hash": null
        }));
    });

    let client = Client::new(&server.base_url(), insecure_config()).expect("client");
    let result = client
        .calculate_market_price(token_id, side, amount, order_type)
        .await;
    mock.assert();
    result
}

#[tokio::test]
async fn buy_price_fok_sufficient_depth() {
    let price = calculate_market_price(
        U256::from(201_u64),
        Side::Buy,
        dec("100"),
        OrderType::Fok,
        vec![],
        vec![
            order_summary("0.5", "100"),
            order_summary("0.4", "100"),
            order_summary("0.3", "100"),
        ],
    )
    .await
    .expect("price");

    assert_eq!(price, dec("0.5"));
}

#[tokio::test]
async fn buy_price_fok_insufficient_depth() {
    let error = calculate_market_price(
        U256::from(202_u64),
        Side::Buy,
        dec("100"),
        OrderType::Fok,
        vec![],
        vec![order_summary("0.5", "100"), order_summary("0.4", "100")],
    )
    .await
    .expect_err("insufficient depth should fail");

    assert!(error.to_string().contains("no match"));
}

#[tokio::test]
async fn buy_price_fak_insufficient_depth() {
    let price = calculate_market_price(
        U256::from(203_u64),
        Side::Buy,
        dec("100"),
        OrderType::Fak,
        vec![],
        vec![order_summary("0.5", "100"), order_summary("0.4", "100")],
    )
    .await
    .expect("price");

    assert_eq!(price, dec("0.5"));
}

#[tokio::test]
async fn sell_price_fok_sufficient_depth() {
    let price = calculate_market_price(
        U256::from(204_u64),
        Side::Sell,
        dec("100"),
        OrderType::Fok,
        vec![
            order_summary("0.3", "100"),
            order_summary("0.4", "100"),
            order_summary("0.5", "100"),
        ],
        vec![],
    )
    .await
    .expect("price");

    assert_eq!(price, dec("0.5"));
}

#[tokio::test]
async fn sell_price_fok_insufficient_depth() {
    let error = calculate_market_price(
        U256::from(205_u64),
        Side::Sell,
        dec("100"),
        OrderType::Fok,
        vec![order_summary("0.4", "10"), order_summary("0.5", "10")],
        vec![],
    )
    .await
    .expect_err("insufficient depth should fail");

    assert!(error.to_string().contains("no match"));
}

#[tokio::test]
async fn sell_price_fak_insufficient_depth() {
    let price = calculate_market_price(
        U256::from(206_u64),
        Side::Sell,
        dec("100"),
        OrderType::Fak,
        vec![order_summary("0.4", "10"), order_summary("0.5", "10")],
        vec![],
    )
    .await
    .expect("price");

    assert_eq!(price, dec("0.4"));
}

#[tokio::test]
async fn buy_accumulates_size_times_price() {
    let price = calculate_market_price(
        U256::from(207_u64),
        Side::Buy,
        dec("100"),
        OrderType::Fok,
        vec![],
        vec![
            order_summary("0.5", "100"),
            order_summary("0.4", "200"),
            order_summary("0.3", "100"),
        ],
    )
    .await
    .expect("price");

    assert_eq!(price, dec("0.4"));
}

#[tokio::test]
async fn sell_accumulates_size_only() {
    let price = calculate_market_price(
        U256::from(208_u64),
        Side::Sell,
        dec("600"),
        OrderType::Fok,
        vec![
            order_summary("0.3", "334"),
            order_summary("0.4", "100"),
            order_summary("0.5", "1000"),
        ],
        vec![],
    )
    .await
    .expect("price");

    assert_eq!(price, dec("0.5"));
}
