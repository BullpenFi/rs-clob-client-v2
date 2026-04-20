mod common;

use std::str::FromStr as _;

use alloy::primitives::{U256, address};
use httpmock::Method::{DELETE, GET, POST};
use httpmock::MockServer;
use polymarket_clob_client_v2::clob::types::{
    AssetType, BalanceAllowanceRequest, OpenOrdersRequest, OrderType, Side, SignatureTypeV2,
    TickSize,
};
use polymarket_clob_client_v2::clob::{Client, Config};
use polymarket_clob_client_v2::error::Status;
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

fn dec(value: &str) -> Decimal {
    Decimal::from_str(value).expect("decimal")
}

fn sample_open_order_json(id: &str) -> serde_json::Value {
    serde_json::json!({
        "id": id,
        "status": "live",
        "owner": "00000000-0000-0000-0000-000000000000",
        "maker_address": "0xf39fd6e51aad88f6f4ce6ab8827279cfffb92266",
        "market": "market-1",
        "asset_id": "123",
        "side": "BUY",
        "original_size": "10",
        "size_matched": "0",
        "price": "0.45",
        "associate_trades": [],
        "outcome": "YES",
        "created_at": 1_700_000_000,
        "expiration": "0",
        "order_type": "GTC"
    })
}

#[test]
fn client_rejects_non_https_hosts_by_default() {
    let error = Client::new("http://example.com", Config::default()).expect_err("http should fail");
    assert!(
        error
            .to_string()
            .contains("only HTTPS URLs are accepted; set allow_insecure for local dev")
    );
}

#[test]
fn client_allows_non_https_hosts_when_explicitly_enabled() {
    let client = Client::new("http://example.com", insecure_config())
        .expect("insecure http client for local dev");
    assert_eq!(client.host().as_str(), "http://example.com/");
}

#[tokio::test]
async fn public_ok_endpoint_works_against_httpmock() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/ok")
            .header("user-agent", "@polymarket/clob-client");
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
            .header("POLY_ADDRESS", "0xf39fd6e51aad88f6f4ce6ab8827279cfffb92266");
        then.status(200)
            .json_body_obj(&serde_json::json!({ "apiKeys": [] }));
    });

    let client = common::create_authenticated(&server.base_url(), insecure_config()).await;

    let response = client.api_keys().await.expect("api keys");
    mock.assert();
    assert!(response.api_keys.is_empty());
}

#[tokio::test]
async fn create_or_derive_api_key_returns_created_credentials_when_present() {
    let server = MockServer::start();
    let signer = common::signer();

    let create_mock = server.mock(|when, then| {
        when.method(POST).path("/auth/api-key");
        then.status(200).json_body_obj(&serde_json::json!({
            "apiKey": "00000000-0000-0000-0000-000000000000",
            "secret": "c2VjcmV0",
            "passphrase": "passphrase"
        }));
    });
    let derive_mock = server.mock(|when, then| {
        when.method(GET).path("/auth/derive-api-key");
        then.status(200).json_body_obj(&serde_json::json!({
            "apiKey": "11111111-1111-1111-1111-111111111111",
            "secret": "ZGVyaXZlZA==",
            "passphrase": "derived"
        }));
    });

    let client = Client::new(&server.base_url(), insecure_config()).expect("client");
    let credentials = client
        .create_or_derive_api_key(&signer, Some(7))
        .await
        .expect("created credentials");

    create_mock.assert_calls(1);
    derive_mock.assert_calls(0);
    assert_eq!(
        credentials.key().to_string(),
        "00000000-0000-0000-0000-000000000000"
    );
}

#[tokio::test]
async fn create_or_derive_api_key_falls_back_after_empty_create_response() {
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

    let client = Client::new(&server.base_url(), insecure_config()).expect("client");
    let credentials = client
        .create_or_derive_api_key(&signer, Some(7))
        .await
        .expect("fallback credentials");

    create_mock.assert_calls(1);
    derive_mock.assert_calls(1);
    assert_eq!(
        credentials.key().to_string(),
        "00000000-0000-0000-0000-000000000000"
    );
}

#[tokio::test]
async fn create_or_derive_api_key_propagates_create_failures() {
    let server = MockServer::start();
    let signer = common::signer();

    let create_mock = server.mock(|when, then| {
        when.method(POST).path("/auth/api-key");
        then.status(500).body("boom");
    });
    let derive_mock = server.mock(|when, then| {
        when.method(GET).path("/auth/derive-api-key");
        then.status(200).json_body_obj(&serde_json::json!({
            "apiKey": "00000000-0000-0000-0000-000000000000",
            "secret": "c2VjcmV0",
            "passphrase": "passphrase"
        }));
    });

    let client = Client::new(&server.base_url(), insecure_config()).expect("client");
    let error = client
        .create_or_derive_api_key(&signer, Some(7))
        .await
        .expect_err("create failure should be returned");

    create_mock.assert_calls(1);
    derive_mock.assert_calls(0);
    assert!(error.to_string().contains("500"));
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
async fn delete_requests_are_not_retried_on_server_errors() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(DELETE).path("/cancel-all");
        then.status(500).body("boom");
    });

    let client = common::create_authenticated(&server.base_url(), retrying_insecure_config()).await;
    let error = client
        .cancel_all()
        .await
        .expect_err("DELETE should not retry");

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
                        then.status(200)
                            .json_body_obj(&Vec::<serde_json::Value>::new());
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
async fn structured_server_errors_preserve_json_payloads() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET).path("/ok");
        then.status(400).json_body_obj(&serde_json::json!({
            "error": {
                "message": "bad request",
                "code": "INVALID_MARKET"
            },
            "status": 400
        }));
    });

    let client = Client::new(&server.base_url(), insecure_config()).expect("client");
    let error = client.ok().await.expect_err("request should fail");
    let status = error.downcast_ref::<Status>().expect("status error");

    mock.assert();
    assert_eq!(status.status_code, reqwest::StatusCode::BAD_REQUEST);
    assert_eq!(
        status.body,
        Some(serde_json::json!({
            "error": {
                "message": "bad request",
                "code": "INVALID_MARKET"
            },
            "status": 400
        }))
    );
    assert_eq!(
        status
            .raw_body
            .as_deref()
            .map(serde_json::from_str::<serde_json::Value>)
            .transpose()
            .expect("raw body should be valid json"),
        Some(serde_json::json!({
            "error": {
                "message": "bad request",
                "code": "INVALID_MARKET"
            },
            "status": 400
        }))
    );
    let rendered = error.to_string();
    assert!(rendered.contains("bad request"));
    assert!(rendered.contains("INVALID_MARKET"));
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

    assert!(
        error
            .to_string()
            .contains("funder address is not supported with EOA signature type")
    );
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

    assert!(
        error
            .to_string()
            .contains("non-zero funder address is required for Proxy/GnosisSafe signature types")
    );
}

#[tokio::test]
async fn calculate_market_price_matches_ts_buy_cutoff_logic() {
    let server = MockServer::start();
    let token_id = U256::from(123_u64);

    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/book")
            .query_param("token_id", "123");
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
        when.method(GET)
            .path("/book")
            .query_param("token_id", "456");
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

#[tokio::test]
async fn server_time_returns_timestamp() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET).path("/time");
        then.status(200).body("1700000000");
    });

    let client = Client::new(&server.base_url(), insecure_config()).expect("client");
    let timestamp = client.server_time().await.expect("server time");

    mock.assert();
    assert_eq!(timestamp, 1_700_000_000);
}

#[tokio::test]
async fn version_returns_and_caches() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET).path("/version");
        then.status(200)
            .json_body_obj(&serde_json::json!({ "version": 7 }));
    });

    let client = Client::new(&server.base_url(), insecure_config()).expect("client");
    let first = client.version().await.expect("version");
    let second = client.version().await.expect("cached version");

    mock.assert_calls(1);
    assert_eq!(first, 7);
    assert_eq!(second, 7);
}

#[tokio::test]
async fn midpoint_returns_decimal() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/midpoint")
            .query_param("token_id", "321");
        then.status(200)
            .json_body_obj(&serde_json::json!({ "mid": "0.53" }));
    });

    let client = Client::new(&server.base_url(), insecure_config()).expect("client");
    let midpoint = client
        .midpoint(U256::from(321_u64))
        .await
        .expect("midpoint");

    mock.assert();
    assert_eq!(midpoint.mid, dec("0.53"));
}

#[tokio::test]
async fn price_returns_decimal() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/price")
            .query_param("token_id", "322")
            .query_param("side", "BUY");
        then.status(200)
            .json_body_obj(&serde_json::json!({ "price": "0.61" }));
    });

    let client = Client::new(&server.base_url(), insecure_config()).expect("client");
    let price = client
        .price(U256::from(322_u64), Side::Buy)
        .await
        .expect("price");

    mock.assert();
    assert_eq!(price.price, dec("0.61"));
}

#[tokio::test]
async fn order_book_returns_bids_asks() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/book")
            .query_param("token_id", "323");
        then.status(200).json_body_obj(&serde_json::json!({
            "market": "market-1",
            "asset_id": "323",
            "timestamp": "1700000000",
            "bids": [{ "price": "0.45", "size": "100" }],
            "asks": [{ "price": "0.55", "size": "100" }],
            "min_order_size": "1",
            "tick_size": "0.01",
            "neg_risk": false,
            "hash": null
        }));
    });

    let client = Client::new(&server.base_url(), insecure_config()).expect("client");
    let book = client
        .order_book(U256::from(323_u64))
        .await
        .expect("order book");

    mock.assert();
    assert_eq!(book.bids.len(), 1);
    assert_eq!(book.asks.len(), 1);
    assert_eq!(book.bids[0].price, dec("0.45"));
    assert_eq!(book.asks[0].price, dec("0.55"));
}

#[tokio::test]
async fn post_order_deserializes_camel_case() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(POST).path("/order");
        then.status(200).json_body_obj(&serde_json::json!({
            "success": true,
            "errorMsg": "",
            "orderID": "order-1",
            "transactionsHashes": ["0xabc"],
            "status": "live",
            "takingAmount": "100",
            "makingAmount": "45"
        }));
    });

    let signer = common::signer();
    let client = common::create_authenticated(&server.base_url(), insecure_config()).await;
    let token_id = U256::from(324_u64);
    client.set_tick_size(token_id, TickSize::Hundredth);
    client.set_neg_risk(token_id, false);

    let signed = client
        .create_order(
            &signer,
            polymarket_clob_client_v2::clob::UserOrder::builder()
                .token_id(token_id)
                .price(dec("0.45"))
                .size(dec("10"))
                .side(Side::Buy)
                .build(),
        )
        .await
        .expect("signed order");

    let response = client.post_order(&signed).await.expect("post order");

    mock.assert();
    assert!(response.success);
    assert_eq!(response.order_id, "order-1");
    assert_eq!(response.transactions_hashes, vec!["0xabc".to_owned()]);
    assert_eq!(response.taking_amount, "100");
    assert_eq!(response.making_amount, "45");
}

#[tokio::test]
async fn cancel_order_sends_order_id() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(DELETE)
            .path("/order")
            .json_body_obj(&serde_json::json!({ "orderID": "order-1" }));
        then.status(200)
            .json_body_obj(&serde_json::json!({ "canceled": true }));
    });

    let client = common::create_authenticated(&server.base_url(), insecure_config()).await;
    let response = client
        .cancel_order("order-1".to_owned())
        .await
        .expect("cancel order");

    mock.assert();
    assert_eq!(response["canceled"], true);
}

#[tokio::test]
async fn cancel_all_sends_empty_body() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(DELETE).path("/cancel-all").body("");
        then.status(200)
            .json_body_obj(&serde_json::json!({ "canceled": true }));
    });

    let client = common::create_authenticated(&server.base_url(), insecure_config()).await;
    let response = client.cancel_all().await.expect("cancel all");

    mock.assert();
    assert_eq!(response["canceled"], true);
}

#[tokio::test]
async fn pagination_collects_all_pages() {
    let server = MockServer::start();
    let first = server.mock(|when, then| {
        when.method(GET)
            .path("/data/orders")
            .query_param("next_cursor", "MA==");
        then.status(200).json_body_obj(&serde_json::json!({
            "limit": 1,
            "count": 2,
            "next_cursor": "MQ==",
            "data": [sample_open_order_json("order-1")]
        }));
    });
    let second = server.mock(|when, then| {
        when.method(GET)
            .path("/data/orders")
            .query_param("next_cursor", "MQ==");
        then.status(200).json_body_obj(&serde_json::json!({
            "limit": 1,
            "count": 2,
            "next_cursor": "LTE=",
            "data": [sample_open_order_json("order-2")]
        }));
    });

    let client = common::create_authenticated(&server.base_url(), insecure_config()).await;
    let orders = client
        .open_orders(
            &OpenOrdersRequest::builder()
                .market("market-1".to_owned())
                .build(),
        )
        .await
        .expect("open orders");

    first.assert_calls(1);
    second.assert_calls(1);
    assert_eq!(orders.len(), 2);
    assert_eq!(orders[0].id, "order-1");
    assert_eq!(orders[1].id, "order-2");
}

#[tokio::test]
async fn pagination_stops_on_empty_page() {
    let server = MockServer::start();
    let first = server.mock(|when, then| {
        when.method(GET)
            .path("/data/orders")
            .query_param("next_cursor", "MA==");
        then.status(200).json_body_obj(&serde_json::json!({
            "limit": 1,
            "count": 0,
            "next_cursor": "MQ==",
            "data": []
        }));
    });
    let second = server.mock(|when, then| {
        when.method(GET)
            .path("/data/orders")
            .query_param("next_cursor", "MQ==");
        then.status(200).json_body_obj(&serde_json::json!({
            "limit": 1,
            "count": 1,
            "next_cursor": "LTE=",
            "data": [sample_open_order_json("order-should-not-load")]
        }));
    });

    let client = common::create_authenticated(&server.base_url(), insecure_config()).await;
    let orders = client
        .open_orders(
            &OpenOrdersRequest::builder()
                .market("market-1".to_owned())
                .build(),
        )
        .await
        .expect("open orders");

    first.assert_calls(1);
    second.assert_calls(0);
    assert!(orders.is_empty());
}

#[tokio::test]
async fn balance_allowance_returns_values() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET).path("/balance-allowance");
        then.status(200).json_body_obj(&serde_json::json!({
            "balance": "100.5",
            "allowance": "75.25"
        }));
    });

    let client = common::create_authenticated(&server.base_url(), insecure_config()).await;
    let response = client
        .balance_allowance(
            &BalanceAllowanceRequest::builder()
                .asset_type(AssetType::Collateral)
                .build(),
        )
        .await
        .expect("balance allowance");

    mock.assert();
    assert_eq!(response.balance, "100.5");
    assert_eq!(response.allowance, "75.25");
}
