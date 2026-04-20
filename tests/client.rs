mod common;

use std::str::FromStr as _;

use alloy::primitives::{U256, address};
use httpmock::Method::{GET, POST};
use httpmock::MockServer;
use polymarket_clob_client_v2::clob::{Client, Config};
use polymarket_clob_client_v2::clob::types::{OrderType, Side, SignatureTypeV2};
use polymarket_clob_client_v2::types::Decimal;
use tokio::time::{Duration, sleep};

fn insecure_config() -> Config {
    Config::builder().allow_insecure(true).build()
}

fn retrying_insecure_config() -> Config {
    Config::builder()
        .allow_insecure(true)
        .retry_on_error(true)
        .build()
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
        when.method(GET)
            .path("/ok")
            .header("user-agent", "polymarket-clob-client-v2");
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

    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/auth/api-keys")
            .header("POLY_API_KEY", "00000000-0000-0000-0000-000000000000")
            .header(
                "POLY_ADDRESS",
                "0xf39fd6e51aad88f6f4ce6ab8827279cfffb92266",
            );
        then.status(200)
            .json_body_obj(&serde_json::json!({ "apiKeys": [] }));
    });

    let client = common::create_authenticated(&server.base_url(), insecure_config()).await;

    let response = client.api_keys().await.expect("api keys");
    mock.assert();
    assert!(response.api_keys.is_empty());
}

#[tokio::test]
async fn create_or_derive_api_key_falls_back_after_non_status_create_failure() {
    let server = MockServer::start();
    let signer = common::signer();

    let create_mock = server.mock(|when, then| {
        when.method(POST).path("/auth/api-key");
        then.status(200).json_body_obj(&serde_json::json!({}));
    });
    let derive_mock = server.mock(|when, then| {
        when.method(GET).path("/auth/derive-api-key");
        then.status(200).json_body_obj(&serde_json::json!({
            "apiKey": "00000000-0000-0000-0000-000000000000",
            "secret": "c2VjcmV0",
            "passphrase": "passphrase"
        }));
    });

    let client = Client::new(&server.base_url(), retrying_insecure_config()).expect("client");
    let credentials = client
        .create_or_derive_api_key(&signer, Some(7))
        .await
        .expect("fallback credentials");

    create_mock.assert_calls(1);
    derive_mock.assert_calls(1);
    assert_eq!(credentials.key().to_string(), "00000000-0000-0000-0000-000000000000");
}

#[tokio::test]
async fn get_requests_are_not_retried_on_server_errors() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET).path("/ok");
        then.status(500).body("boom");
    });

    let client = Client::new(&server.base_url(), retrying_insecure_config()).expect("client");
    let error = client.ok().await.expect_err("GET should not retry");

    mock.assert_calls(1);
    assert!(error.to_string().contains("500"));
}

#[tokio::test]
async fn post_requests_retry_once_on_server_errors() {
    let server = MockServer::start_async().await;
    let first = server
        .mock_async(|when, then| {
            when.method(POST).path("/books");
            then.status(500).body("boom");
        })
        .await;

    let client = Client::new(&server.base_url(), retrying_insecure_config()).expect("client");
    let switch_mock = async {
        loop {
            if first.calls_async().await == 1 {
                first.delete_async().await;
                return server
                    .mock_async(|when, then| {
                        when.method(POST).path("/books");
                        then.status(200).json_body_obj(&Vec::<serde_json::Value>::new());
                    })
                    .await;
            }

            sleep(Duration::from_millis(5)).await;
        }
    };

    let (second, response) = tokio::join!(switch_mock, client.order_books(&[]));
    let response = response.expect("POST should retry once and succeed");

    second.assert_calls(1);
    assert!(response.is_empty());
}

#[tokio::test]
async fn authenticate_rejects_eoa_with_funder() {
    let signer = common::signer();

    let client = Client::new("https://example.com", Config::default()).expect("client");
    let error = client
        .authentication_builder(&signer)
        .credentials(common::credentials())
        .funder(address!("0x0000000000000000000000000000000000000001"))
        .authenticate()
        .await
        .expect_err("EOA with funder should fail");

    assert!(error
        .to_string()
        .contains("funder address is not supported with EOA signature type"));
}

#[tokio::test]
async fn authenticate_rejects_proxy_without_non_zero_funder() {
    let signer = common::signer();

    let client = Client::new("https://example.com", Config::default()).expect("client");
    let error = client
        .authentication_builder(&signer)
        .credentials(common::credentials())
        .signature_type(SignatureTypeV2::Proxy)
        .authenticate()
        .await
        .expect_err("Proxy without funder should fail");

    assert!(error
        .to_string()
        .contains("non-zero funder address is required for Proxy/GnosisSafe signature types"));
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
