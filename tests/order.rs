use std::str::FromStr as _;

use alloy::primitives::U256;
use alloy::signers::Signer as _;
use httpmock::Method::GET;
use httpmock::MockServer;
use polymarket_clob_client_v2::auth::{Credentials, PrivateKeySigner};
use polymarket_clob_client_v2::clob::{Client, Config, UserMarketOrder};
use polymarket_clob_client_v2::clob::types::{BuilderConfig, FeeInfo, Side, TickSize};
use polymarket_clob_client_v2::POLYGON;
use polymarket_clob_client_v2::types::Decimal;
use uuid::Uuid;

fn signer() -> PrivateKeySigner {
    PrivateKeySigner::from_str(
        "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80",
    )
    .expect("valid private key")
    .with_chain_id(Some(POLYGON))
}

#[tokio::test]
async fn limit_order_builder_matches_v2_amount_rules() {
    let signer = signer();
    let client = Client::new("https://clob.polymarket.com", Config::default())
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

    let token_id = U256::from(123_u64);
    client.set_tick_size(token_id, TickSize::Hundredth);
    client.set_neg_risk(token_id, false);

    let signable = client
        .limit_order()
        .token_id(token_id)
        .price(Decimal::from_str("0.56").expect("decimal"))
        .size(Decimal::from_str("21.04").expect("decimal"))
        .side(Side::Buy)
        .build()
        .await
        .expect("signable order");

    assert_eq!(signable.order.makerAmount.to_string(), "11782400");
    assert_eq!(signable.order.takerAmount.to_string(), "21040000");

    let signed = client.sign(&signer, signable).await.expect("signed order");
    assert_eq!(signed.order.signer, signer.address());
}

#[tokio::test]
async fn create_market_order_adjusts_buy_amount_for_builder_fees() {
    const BUILDER_CODE: &str =
        "0x1111111111111111111111111111111111111111111111111111111111111111";

    let server = MockServer::start();
    let builder_fee_mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!("/fees/builder-fees/{BUILDER_CODE}"));
        then.status(200).json_body_obj(&serde_json::json!({
            "builder_maker_fee_rate_bps": 0,
            "builder_taker_fee_rate_bps": 200
        }));
    });

    let config = Config::builder()
        .allow_insecure(true)
        .builder(
            BuilderConfig::builder()
                .builder_code(BUILDER_CODE.to_owned())
                .build(),
        )
        .build();
    let signer = signer();
    let client = Client::new(&server.base_url(), config)
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

    let token_id = U256::from(456_u64);
    client.set_tick_size(token_id, TickSize::Hundredth);
    client.set_neg_risk(token_id, false);
    client.set_fee_info(
        token_id,
        FeeInfo::builder()
            .rate(Decimal::ZERO)
            .exponent(0)
            .build(),
    );

    let signed = client
        .create_market_order(
            &signer,
            UserMarketOrder::builder()
                .token_id(token_id)
                .price(Decimal::from_str("0.50").expect("decimal"))
                .amount(Decimal::from_str("100").expect("decimal"))
                .side(Side::Buy)
                .user_usdc_balance(Decimal::from_str("100").expect("decimal"))
                .build(),
        )
        .await
        .expect("signed order");

    builder_fee_mock.assert();
    assert_eq!(signed.order.makerAmount.to_string(), "98030000");
    assert_eq!(signed.order.takerAmount.to_string(), "196060000");
}
