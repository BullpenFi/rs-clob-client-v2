mod common;

use std::str::FromStr as _;

use alloy::primitives::U256;
use httpmock::Method::GET;
use httpmock::MockServer;
use polymarket_clob_client_v2::clob::types::{BuilderConfig, FeeInfo, Side, TickSize};
use polymarket_clob_client_v2::clob::{Config, UserMarketOrder};
use polymarket_clob_client_v2::types::Decimal;

fn dec(value: &str) -> Decimal {
    Decimal::from_str(value).expect("decimal")
}

fn calculate_platform_fee(
    amount_usd: Decimal,
    price: Decimal,
    fee_rate: Decimal,
    fee_exponent: u32,
) -> Decimal {
    let price_term = price * (Decimal::ONE - price);
    let mut price_term_power = Decimal::ONE;
    for _ in 0..fee_exponent {
        price_term_power *= price_term;
    }

    (amount_usd / price) * fee_rate * price_term_power
}

async fn configured_client(
    host: &str,
    config: Config,
    token_id: U256,
    fee_rate: Decimal,
    fee_exponent: u32,
) -> polymarket_clob_client_v2::clob::Client<
    polymarket_clob_client_v2::auth::state::Authenticated<polymarket_clob_client_v2::auth::Normal>,
> {
    let client = common::create_authenticated(host, config).await;
    client.set_tick_size(token_id, TickSize::Hundredth);
    client.set_neg_risk(token_id, false);
    client.set_fee_info(
        token_id,
        FeeInfo::builder()
            .rate(fee_rate)
            .exponent(fee_exponent)
            .build(),
    );
    client
}

#[test]
fn platform_fee_at_price_0_5() {
    assert_eq!(
        calculate_platform_fee(dec("100"), dec("0.5"), dec("0.02"), 1),
        dec("1.0")
    );
}

#[test]
fn platform_fee_at_price_0_1() {
    assert_eq!(
        calculate_platform_fee(dec("100"), dec("0.1"), dec("0.02"), 1),
        dec("1.8")
    );
}

#[test]
fn platform_fee_at_price_0_99() {
    assert_eq!(
        calculate_platform_fee(dec("100"), dec("0.99"), dec("0.02"), 1),
        dec("0.02")
    );
}

#[tokio::test]
async fn builder_fee_conversion_bps() {
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

    let token_id = U256::from(301_u64);
    let config = Config::builder()
        .allow_insecure(true)
        .builder(
            BuilderConfig::builder()
                .builder_code(BUILDER_CODE.to_owned())
                .build(),
        )
        .build();
    let signer = common::signer();
    let client = configured_client(&server.base_url(), config, token_id, Decimal::ZERO, 0).await;

    let signed = client
        .create_market_order(
            &signer,
            UserMarketOrder::builder()
                .token_id(token_id)
                .price(dec("0.50"))
                .amount(dec("100"))
                .side(Side::Buy)
                .user_usdc_balance(dec("100"))
                .build(),
        )
        .await
        .expect("signed order");

    builder_fee_mock.assert();
    assert_eq!(signed.order.makerAmount.to_string(), "98030000");
    assert_eq!(signed.order.takerAmount.to_string(), "196060000");
}

#[tokio::test]
async fn adjust_buy_amount_no_adjustment() {
    let token_id = U256::from(302_u64);
    let signer = common::signer();
    let client = configured_client(
        common::TEST_HOST,
        Config::default(),
        token_id,
        dec("0.02"),
        1,
    )
    .await;

    let signed = client
        .create_market_order(
            &signer,
            UserMarketOrder::builder()
                .token_id(token_id)
                .price(dec("0.50"))
                .amount(dec("100"))
                .side(Side::Buy)
                .user_usdc_balance(dec("102"))
                .build(),
        )
        .await
        .expect("signed order");

    assert_eq!(signed.order.makerAmount.to_string(), "100000000");
    assert_eq!(signed.order.takerAmount.to_string(), "200000000");
}

#[tokio::test]
async fn adjust_buy_amount_with_adjustment() {
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

    let token_id = U256::from(303_u64);
    let config = Config::builder()
        .allow_insecure(true)
        .builder(
            BuilderConfig::builder()
                .builder_code(BUILDER_CODE.to_owned())
                .build(),
        )
        .build();
    let signer = common::signer();
    let client = configured_client(&server.base_url(), config, token_id, dec("0.02"), 1).await;

    let signed = client
        .create_market_order(
            &signer,
            UserMarketOrder::builder()
                .token_id(token_id)
                .price(dec("0.50"))
                .amount(dec("100"))
                .side(Side::Buy)
                .user_usdc_balance(dec("100"))
                .build(),
        )
        .await
        .expect("signed order");

    builder_fee_mock.assert();
    assert_eq!(signed.order.makerAmount.to_string(), "97080000");
    assert_eq!(signed.order.takerAmount.to_string(), "194160000");
}

#[tokio::test]
async fn adjust_buy_amount_zero_builder_fee() {
    let token_id = U256::from(304_u64);
    let signer = common::signer();
    let client = configured_client(
        common::TEST_HOST,
        Config::default(),
        token_id,
        dec("0.02"),
        1,
    )
    .await;

    let signed = client
        .create_market_order(
            &signer,
            UserMarketOrder::builder()
                .token_id(token_id)
                .price(dec("0.50"))
                .amount(dec("100"))
                .side(Side::Buy)
                .user_usdc_balance(dec("100"))
                .build(),
        )
        .await
        .expect("signed order");

    assert_eq!(signed.order.makerAmount.to_string(), "99000000");
    assert_eq!(signed.order.takerAmount.to_string(), "198000000");
}
