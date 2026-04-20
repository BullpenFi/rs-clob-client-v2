mod common;

use std::str::FromStr as _;

use alloy::primitives::U256;
use httpmock::Method::GET;
use httpmock::MockServer;
use polymarket_clob_client_v2::clob::types::{BuilderConfig, FeeInfo, Side, TickSize};
use polymarket_clob_client_v2::clob::{Config, UserMarketOrder};
use polymarket_clob_client_v2::types::Decimal;

#[tokio::test]
async fn limit_order_builder_matches_v2_amount_rules() {
    let signer = common::signer();
    let client = common::create_authenticated(common::TEST_HOST, Config::default()).await;

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
async fn limit_order_builder_rejects_price_decimal_places_smaller_than_tick_size() {
    let client = common::create_authenticated(common::TEST_HOST, Config::default()).await;

    let token_id = U256::from(124_u64);
    client.set_tick_size(token_id, TickSize::Hundredth);
    client.set_neg_risk(token_id, false);

    let error = client
        .limit_order()
        .token_id(token_id)
        .price(Decimal::from_str("0.345").expect("decimal"))
        .size(Decimal::from_str("10").expect("decimal"))
        .side(Side::Buy)
        .build()
        .await
        .expect_err("price precision should fail");

    assert!(
        error
            .to_string()
            .contains("price has too many decimal places for tick size")
    );
}

#[tokio::test]
async fn create_market_order_adjusts_buy_amount_for_builder_fees() {
    const BUILDER_CODE: &str = "0x1111111111111111111111111111111111111111111111111111111111111111";

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
    let signer = common::signer();
    let client = common::create_authenticated(&server.base_url(), config).await;

    let token_id = U256::from(456_u64);
    client.set_tick_size(token_id, TickSize::Hundredth);
    client.set_neg_risk(token_id, false);
    client.set_fee_info(
        token_id,
        FeeInfo::builder().rate(Decimal::ZERO).exponent(0).build(),
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
