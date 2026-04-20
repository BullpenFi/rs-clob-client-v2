use std::str::FromStr as _;

use alloy::primitives::U256;
use alloy::signers::Signer as _;
use httpmock::Method::GET;
use httpmock::MockServer;
use polymarket_clob_client_v2::auth::{Credentials, PrivateKeySigner};
use polymarket_clob_client_v2::clob::{Client, Config};
use polymarket_clob_client_v2::clob::types::{OrderType, Side};
use polymarket_clob_client_v2::types::Decimal;
use uuid::Uuid;

fn insecure_config() -> Config {
    Config::builder().allow_insecure(true).build()
}

#[test]
fn client_rejects_non_https_hosts_by_default() {
    let error = Client::new("http://example.com", Config::default()).expect_err("http should fail");
    assert!(error
        .to_string()
        .contains("only HTTPS URLs are accepted; set allow_insecure for local dev"));
}

#[test]
fn client_allows_non_https_hosts_when_explicitly_enabled() {
    let client =
        Client::new("http://example.com", insecure_config()).expect("insecure http client for local dev");
    assert_eq!(client.host().as_str(), "http://example.com/");
}

#[tokio::test]
async fn public_ok_endpoint_works_against_httpmock() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET).path("/ok");
        then.status(200).body("OK");
    });

    let client = Client::new(&server.base_url(), insecure_config()).expect("client");
    let response = client.ok().await.expect("ok response");

    mock.assert();
    assert_eq!(response, "OK");
}

#[tokio::test]
async fn authenticated_api_keys_endpoint_signs_l2_headers() {
    let server = MockServer::start();
    let signer = PrivateKeySigner::from_str(
        "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80",
    )
    .expect("valid private key")
    .with_chain_id(Some(polymarket_clob_client_v2::POLYGON));

    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/auth/api-keys")
            .header("POLY_API_KEY", "00000000-0000-0000-0000-000000000000");
        then.status(200)
            .json_body_obj(&serde_json::json!({ "apiKeys": [] }));
    });

    let client = Client::new(&server.base_url(), insecure_config())
        .expect("client")
        .authentication_builder(&signer)
        .credentials(Credentials::new(
            Uuid::nil(),
            "c2VjcmV0".to_owned(),
            "passphrase".to_owned(),
        ))
        .authenticate()
        .await
        .expect("authenticated client");

    let response = client.api_keys().await.expect("api keys");
    mock.assert();
    assert!(response.api_keys.is_empty());
}

#[tokio::test]
async fn calculate_market_price_matches_ts_buy_cutoff_logic() {
    let server = MockServer::start();
    let token_id = U256::from(123_u64);

    let mock = server.mock(|when, then| {
        when.method(GET).path("/book").query_param("token_id", "123");
        then.status(200).json_body_obj(&serde_json::json!({
            "market": "market-1",
            "asset_id": "123",
            "timestamp": "1700000000",
            "bids": [],
            "asks": [
                { "price": "0.50", "size": "50" },
                { "price": "0.45", "size": "50" },
                { "price": "0.40", "size": "50" }
            ],
            "min_order_size": "1",
            "tick_size": "0.01",
            "neg_risk": false,
            "hash": null
        }));
    });

    let client = Client::new(&server.base_url(), insecure_config()).expect("client");
    let price = client
        .calculate_market_price(
            token_id,
            Side::Buy,
            Decimal::from_str("40").expect("decimal"),
            OrderType::Fok,
        )
        .await
        .expect("market price");

    mock.assert();
    assert_eq!(price, Decimal::from_str("0.45").expect("decimal"));
}

#[tokio::test]
async fn calculate_market_price_matches_ts_sell_cutoff_logic() {
    let server = MockServer::start();
    let token_id = U256::from(456_u64);

    let mock = server.mock(|when, then| {
        when.method(GET).path("/book").query_param("token_id", "456");
        then.status(200).json_body_obj(&serde_json::json!({
            "market": "market-2",
            "asset_id": "456",
            "timestamp": "1700000000",
            "bids": [
                { "price": "0.30", "size": "50" },
                { "price": "0.40", "size": "100" },
                { "price": "0.50", "size": "50" }
            ],
            "asks": [],
            "min_order_size": "1",
            "tick_size": "0.01",
            "neg_risk": false,
            "hash": null
        }));
    });

    let client = Client::new(&server.base_url(), insecure_config()).expect("client");
    let price = client
        .calculate_market_price(
            token_id,
            Side::Sell,
            Decimal::from_str("120").expect("decimal"),
            OrderType::Fok,
        )
        .await
        .expect("market price");

    mock.assert();
    assert_eq!(price, Decimal::from_str("0.40").expect("decimal"));
}
