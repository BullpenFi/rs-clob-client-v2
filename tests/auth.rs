mod common;

use std::str::FromStr as _;

use alloy::hex::ToHexExt as _;
use alloy::primitives::{B256, U256, address};
use httpmock::Method::{GET, POST};
use httpmock::MockServer;
use polymarket_clob_client_v2::auth::builder;
use polymarket_clob_client_v2::auth::state::Authenticated;
use polymarket_clob_client_v2::auth::{Credentials, Normal, PrivateKeySigner, Signer as _, l1, l2};
use polymarket_clob_client_v2::clob::types::{Order, new_order};
use polymarket_clob_client_v2::clob::types::{Side, SignatureTypeV2, sign_order, signing_hash};
use polymarket_clob_client_v2::clob::{Client, Config};
use polymarket_clob_client_v2::{AMOY, POLYGON, config};
use reqwest::Method;
use reqwest::header::HeaderValue;
use uuid::Uuid;

fn sample_order() -> Order {
    new_order(
        U256::from(1_u64),
        address!("0x0000000000000000000000000000000000000001"),
        common::signer().address(),
        U256::from(123_u64),
        U256::from(1_000_000_u64),
        U256::from(2_000_000_u64),
        Side::Buy,
        SignatureTypeV2::Eoa,
        1_700_000_000_000,
        B256::ZERO,
        B256::ZERO,
    )
}

fn vector_credentials() -> Credentials {
    Credentials::new(
        Uuid::nil(),
        "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=".to_owned(),
        common::PASSPHRASE.to_owned(),
    )
}

fn request(method: Method, url: &str, body: Option<&str>) -> reqwest::Request {
    let client = reqwest::Client::new();
    let mut builder = client.request(method, url);
    if let Some(body) = body {
        builder = builder.body(body.to_owned());
    }
    builder.build().expect("request")
}

#[tokio::test]
async fn creates_l1_headers() {
    let signer = common::signer();
    let headers = l1::create_headers(&signer, POLYGON, 1_700_000_000, Some(7))
        .await
        .expect("l1 headers");

    assert_eq!(
        headers.get(l1::POLY_NONCE),
        Some(&HeaderValue::from_static("7"))
    );
    assert_eq!(
        headers.get(l1::POLY_TIMESTAMP),
        Some(&HeaderValue::from_static("1700000000"))
    );
    assert_eq!(
        headers.get(l1::POLY_ADDRESS),
        Some(
            &HeaderValue::from_str(&signer.address().encode_hex_with_prefix())
                .expect("header value"),
        )
    );
    assert!(headers.get(l1::POLY_SIGNATURE).is_some());
}

#[tokio::test]
async fn l1_headers_match_ts_amoy_vector() {
    let signer = PrivateKeySigner::from_str(common::PRIVATE_KEY)
        .expect("valid private key")
        .with_chain_id(Some(AMOY));

    let headers = l1::create_headers(&signer, AMOY, 10_000_000, Some(23))
        .await
        .expect("l1 headers");

    assert_eq!(
        headers.get(l1::POLY_SIGNATURE),
        Some(&HeaderValue::from_static(
            "0xf62319a987514da40e57e2f4d7529f7bac38f0355bd88bb5adbb3768d80de6c1682518e0af677d5260366425f4361e7b70c25ae232aff0ab2331e2b164a1aedc1b"
        ))
    );
}

#[tokio::test]
async fn creates_l2_headers() {
    let signer = common::signer();
    let state = Authenticated::new(signer.address(), common::credentials(), Normal::new());

    let request = reqwest::Client::new()
        .get("https://example.com/data/orders")
        .build()
        .expect("request");

    let headers = l2::create_headers(&state, &request, 42)
        .await
        .expect("l2 headers");

    assert_eq!(
        headers.get(l2::POLY_API_KEY),
        Some(&HeaderValue::from_static(
            "00000000-0000-0000-0000-000000000000"
        ))
    );
    assert_eq!(
        headers.get(l2::POLY_PASSPHRASE),
        Some(&HeaderValue::from_static("passphrase"))
    );
    assert_eq!(
        headers.get(l2::POLY_TIMESTAMP),
        Some(&HeaderValue::from_static("42"))
    );
    assert_eq!(
        headers.get(l2::POLY_ADDRESS),
        Some(
            &HeaderValue::from_str(&signer.address().encode_hex_with_prefix())
                .expect("header value"),
        )
    );
    assert!(headers.get(l2::POLY_SIGNATURE).is_some());
}

#[tokio::test]
async fn l2_hmac_matches_ts_known_vector() {
    let signer = common::signer();
    let state = Authenticated::new(signer.address(), vector_credentials(), Normal::new());
    let request = request(
        Method::from_bytes(b"test-sign").expect("custom method"),
        "https://example.com/orders",
        Some("{\"hash\": \"0x123\"}"),
    );

    let headers = l2::create_headers(&state, &request, 1_000_000)
        .await
        .expect("l2 headers");

    assert_eq!(
        headers.get(l2::POLY_SIGNATURE),
        Some(&HeaderValue::from_static(
            "ZwAdJKvoYRlEKDkNMwd5BuwNNtg93kNaR_oU2HrfVvc="
        ))
    );
}

#[tokio::test]
async fn l2_headers_without_body_match_expected_signature() {
    let signer = common::signer();
    let state = Authenticated::new(signer.address(), vector_credentials(), Normal::new());
    let request = request(Method::GET, "https://example.com/order", None);

    let headers = l2::create_headers(&state, &request, 1_000_000)
        .await
        .expect("l2 headers");

    assert_eq!(
        headers.get(l2::POLY_SIGNATURE),
        Some(&HeaderValue::from_static(
            "fBe-xXz0q8hWO7dVwXYL0VVBY3psQE5aj_uoE1hZt08="
        ))
    );
}

#[tokio::test]
async fn l2_headers_with_body_match_expected_signature() {
    let signer = common::signer();
    let state = Authenticated::new(signer.address(), vector_credentials(), Normal::new());
    let request = request(
        Method::POST,
        "https://example.com/orders",
        Some("{\"hash\": \"0x123\"}"),
    );

    let headers = l2::create_headers(&state, &request, 1_000_000)
        .await
        .expect("l2 headers");

    assert_eq!(
        headers.get(l2::POLY_SIGNATURE),
        Some(&HeaderValue::from_static(
            "wdXSC4akzPKG0yFk9FJrIb7-rg73v_M7QDxIBp-P1CQ="
        ))
    );
}

#[tokio::test]
async fn order_signing_round_trip_recovers_signer() {
    let signer = common::signer();
    let order = sample_order();
    let verifying_contract = config::exchange_contract(POLYGON, false).expect("exchange contract");

    let hash = signing_hash(&order, POLYGON, verifying_contract);
    let signature = sign_order(&signer, &order, POLYGON, verifying_contract)
        .await
        .expect("sign order");

    assert_ne!(hash, B256::ZERO);
    assert_eq!(
        signature
            .recover_address_from_prehash(&hash)
            .expect("recover"),
        signer.address()
    );
}

#[test]
fn remote_builder_config_debug_redacts_bearer_token() {
    let config = builder::Config::remote(
        "https://example.com/sign",
        Some("super-secret-token".to_owned()),
    )
    .expect("remote builder config");

    let debug = format!("{config:?}");
    assert!(debug.contains("[REDACTED]"));
    assert!(!debug.contains("super-secret-token"));
}

#[test]
fn remote_builder_config_rejects_http_hosts_by_default() {
    let error = builder::Config::remote("http://example.com/sign", None)
        .expect_err("http should be rejected");

    assert!(error.to_string().contains(
        "only HTTPS URLs are accepted for remote builder signing; use remote_insecure for local dev"
    ));
}

#[tokio::test]
async fn remote_builder_headers_support_explicit_insecure_local_dev_hosts() {
    let signer_server = MockServer::start();
    let api_server = MockServer::start();

    let sign_mock = signer_server.mock(|when, then| {
        when.method(POST)
            .path("/sign")
            .header("authorization", "Bearer builder-token");
        then.status(200).json_body_obj(&serde_json::json!({
            "POLY_BUILDER_API_KEY": "builder-key",
            "POLY_BUILDER_TIMESTAMP": "1700000000",
            "POLY_BUILDER_PASSPHRASE": "builder-passphrase",
            "POLY_BUILDER_SIGNATURE": "builder-signature"
        }));
    });
    let api_mock = api_server.mock(|when, then| {
        when.method(GET)
            .path("/auth/api-keys")
            .header("POLY_BUILDER_API_KEY", "builder-key")
            .header("POLY_BUILDER_PASSPHRASE", "builder-passphrase")
            .header("POLY_BUILDER_SIGNATURE", "builder-signature")
            .header("POLY_BUILDER_TIMESTAMP", "1700000000");
        then.status(200)
            .json_body_obj(&serde_json::json!({ "apiKeys": [] }));
    });

    let client = Client::new(
        &api_server.base_url(),
        Config::builder().allow_insecure(true).build(),
    )
    .expect("client")
    .authentication_builder(&common::signer())
    .credentials(common::credentials())
    .kind(builder::Builder::new(
        builder::Config::remote_insecure(
            &format!("{}/sign", signer_server.base_url()),
            Some("builder-token".to_owned()),
        )
        .expect("remote builder config"),
    ))
    .authenticate()
    .await
    .expect("authenticated client");

    let response = client.api_keys().await.expect("api keys");

    sign_mock.assert();
    api_mock.assert();
    assert!(response.api_keys.is_empty());
}
