use std::str::FromStr as _;

use alloy::primitives::{B256, U256, address};
use alloy::signers::Signer as _;
use polymarket_clob_client_v2::auth::state::Authenticated;
use polymarket_clob_client_v2::auth::{Credentials, Normal, PrivateKeySigner, l1, l2};
use polymarket_clob_client_v2::clob::types::{OrderType, Side, SignatureTypeV2, sign_order, signing_hash};
use polymarket_clob_client_v2::clob::types::{Order, new_order};
use polymarket_clob_client_v2::{POLYGON, config};
use reqwest::header::HeaderValue;
use uuid::Uuid;

fn signer() -> PrivateKeySigner {
    PrivateKeySigner::from_str(
        "0x59c6995e998f97a5a0044976f7ad0e51f4f9c2f6f9b8d264bd67bb0a4d8b8d36",
    )
    .expect("valid private key")
    .with_chain_id(Some(POLYGON))
}

fn sample_order() -> Order {
    new_order(
        U256::from(1_u64),
        address!("0x0000000000000000000000000000000000000001"),
        signer().address(),
        U256::from(123_u64),
        U256::from(1_000_000_u64),
        U256::from(2_000_000_u64),
        Side::Buy,
        SignatureTypeV2::Eoa,
        1_700_000_000_000,
        B256::ZERO,
        B256::ZERO,
        0,
    )
}

#[tokio::test]
async fn creates_l1_headers() {
    let signer = signer();
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
        Some(&HeaderValue::from_str(&signer.address().to_string()).expect("header value"))
    );
    assert!(headers.get(l1::POLY_SIGNATURE).is_some());
}

#[tokio::test]
async fn creates_l2_headers() {
    let signer = signer();
    let state = Authenticated::new(
        signer.address(),
        Credentials::new(Uuid::nil(), "c2VjcmV0".to_owned(), "passphrase".to_owned()),
        Normal::new(),
    );

    let request = reqwest::Client::new()
        .get("https://example.com/data/orders")
        .build()
        .expect("request");

    let headers = l2::create_headers(&state, &request, 42)
        .await
        .expect("l2 headers");

    assert_eq!(
        headers.get(l2::POLY_API_KEY),
        Some(&HeaderValue::from_static("00000000-0000-0000-0000-000000000000"))
    );
    assert_eq!(
        headers.get(l2::POLY_PASSPHRASE),
        Some(&HeaderValue::from_static("passphrase"))
    );
    assert_eq!(
        headers.get(l2::POLY_TIMESTAMP),
        Some(&HeaderValue::from_static("42"))
    );
    assert!(headers.get(l2::POLY_SIGNATURE).is_some());
}

#[tokio::test]
async fn order_signing_round_trip_recovers_signer() {
    let signer = signer();
    let order = sample_order();
    let verifying_contract =
        config::exchange_contract(POLYGON, false).expect("exchange contract");

    let hash = signing_hash(&order, POLYGON, verifying_contract);
    let signature = sign_order(&signer, &order, POLYGON, verifying_contract)
        .await
        .expect("sign order");

    assert_ne!(hash, B256::ZERO);
    assert_eq!(
        signature.recover_address_from_prehash(&hash).expect("recover"),
        signer.address()
    );
}

#[test]
fn signable_order_type_is_constructible() {
    let _ = polymarket_clob_client_v2::clob::types::SignableOrder {
        order: sample_order(),
        order_type: OrderType::Gtc,
        post_only: false,
        defer_exec: false,
    };
}
