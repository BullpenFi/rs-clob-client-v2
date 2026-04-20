mod common;

use std::str::FromStr as _;

use alloy::primitives::{B256, U256};
use polymarket_clob_client_v2::clob::Config;
use polymarket_clob_client_v2::clob::types::{BuilderConfig, Side, TickSize};
use polymarket_clob_client_v2::types::Decimal;

fn dec(value: &str) -> Decimal {
    Decimal::from_str(value).expect("decimal")
}

async fn configured_client(
    token_id: U256,
    tick_size: TickSize,
    config: Config,
) -> polymarket_clob_client_v2::clob::Client<
    polymarket_clob_client_v2::auth::state::Authenticated<polymarket_clob_client_v2::auth::Normal>,
> {
    let client = common::create_authenticated("https://clob.polymarket.com", config).await;
    client.set_tick_size(token_id, tick_size);
    client.set_neg_risk(token_id, false);
    client
}

fn assert_scaled_amounts(
    signable: &polymarket_clob_client_v2::clob::types::SignableOrder,
    maker_amount: &str,
    taker_amount: &str,
) {
    assert_eq!(signable.order.makerAmount.to_string(), maker_amount);
    assert_eq!(signable.order.takerAmount.to_string(), taker_amount);
}

#[tokio::test]
async fn limit_buy_tick_0_1() {
    let token_id = U256::from(101_u64);
    let client = configured_client(token_id, TickSize::Tenth, Config::default()).await;

    let signable = client
        .limit_order()
        .token_id(token_id)
        .price(dec("0.5"))
        .size(dec("100"))
        .side(Side::Buy)
        .build()
        .await
        .expect("signable order");

    assert_scaled_amounts(&signable, "50000000", "100000000");
}

#[tokio::test]
async fn limit_buy_tick_0_01() {
    let token_id = U256::from(102_u64);
    let client = configured_client(token_id, TickSize::Hundredth, Config::default()).await;

    let signable = client
        .limit_order()
        .token_id(token_id)
        .price(dec("0.45"))
        .size(dec("100"))
        .side(Side::Buy)
        .build()
        .await
        .expect("signable order");

    assert_scaled_amounts(&signable, "45000000", "100000000");
}

#[tokio::test]
async fn limit_buy_tick_0_001() {
    let token_id = U256::from(103_u64);
    let client = configured_client(token_id, TickSize::Thousandth, Config::default()).await;

    let signable = client
        .limit_order()
        .token_id(token_id)
        .price(dec("0.456"))
        .size(dec("100"))
        .side(Side::Buy)
        .build()
        .await
        .expect("signable order");

    assert_scaled_amounts(&signable, "45600000", "100000000");
}

#[tokio::test]
async fn limit_buy_tick_0_0001() {
    let token_id = U256::from(104_u64);
    let client = configured_client(token_id, TickSize::TenThousandth, Config::default()).await;

    let signable = client
        .limit_order()
        .token_id(token_id)
        .price(dec("0.4567"))
        .size(dec("100"))
        .side(Side::Buy)
        .build()
        .await
        .expect("signable order");

    assert_scaled_amounts(&signable, "45670000", "100000000");
}

#[tokio::test]
async fn limit_sell_tick_0_1() {
    let token_id = U256::from(105_u64);
    let client = configured_client(token_id, TickSize::Tenth, Config::default()).await;

    let signable = client
        .limit_order()
        .token_id(token_id)
        .price(dec("0.5"))
        .size(dec("50"))
        .side(Side::Sell)
        .build()
        .await
        .expect("signable order");

    assert_scaled_amounts(&signable, "50000000", "25000000");
}

#[tokio::test]
async fn limit_sell_tick_0_01() {
    let token_id = U256::from(106_u64);
    let client = configured_client(token_id, TickSize::Hundredth, Config::default()).await;

    let signable = client
        .limit_order()
        .token_id(token_id)
        .price(dec("0.60"))
        .size(dec("50"))
        .side(Side::Sell)
        .build()
        .await
        .expect("signable order");

    assert_scaled_amounts(&signable, "50000000", "30000000");
}

#[tokio::test]
async fn limit_sell_tick_0_001() {
    let token_id = U256::from(107_u64);
    let client = configured_client(token_id, TickSize::Thousandth, Config::default()).await;

    let signable = client
        .limit_order()
        .token_id(token_id)
        .price(dec("0.600"))
        .size(dec("50"))
        .side(Side::Sell)
        .build()
        .await
        .expect("signable order");

    assert_scaled_amounts(&signable, "50000000", "30000000");
}

#[tokio::test]
async fn limit_sell_tick_0_0001() {
    let token_id = U256::from(108_u64);
    let client = configured_client(token_id, TickSize::TenThousandth, Config::default()).await;

    let signable = client
        .limit_order()
        .token_id(token_id)
        .price(dec("0.6000"))
        .size(dec("50"))
        .side(Side::Sell)
        .build()
        .await
        .expect("signable order");

    assert_scaled_amounts(&signable, "50000000", "30000000");
}

#[tokio::test]
async fn market_buy_tick_0_01() {
    let token_id = U256::from(109_u64);
    let client = configured_client(token_id, TickSize::Hundredth, Config::default()).await;

    let signable = client
        .market_order()
        .token_id(token_id)
        .price(dec("0.50"))
        .amount(dec("100"))
        .side(Side::Buy)
        .build()
        .await
        .expect("signable order");

    assert_scaled_amounts(&signable, "100000000", "200000000");
}

#[tokio::test]
async fn market_sell_tick_0_01() {
    let token_id = U256::from(110_u64);
    let client = configured_client(token_id, TickSize::Hundredth, Config::default()).await;

    let signable = client
        .market_order()
        .token_id(token_id)
        .price(dec("0.60"))
        .amount(dec("50"))
        .side(Side::Sell)
        .build()
        .await
        .expect("signable order");

    assert_scaled_amounts(&signable, "50000000", "30000000");
}

#[tokio::test]
async fn rounding_cascade_triggers() {
    let token_id = U256::from(111_u64);
    let client = configured_client(token_id, TickSize::Hundredth, Config::default()).await;

    let signable = client
        .market_order()
        .token_id(token_id)
        .price(dec("0.56"))
        .amount(dec("1"))
        .side(Side::Buy)
        .build()
        .await
        .expect("signable order");

    assert_scaled_amounts(&signable, "1000000", "1785700");
}

#[tokio::test]
async fn builder_code_propagation() {
    const BUILDER_CODE: &str =
        "0x1111111111111111111111111111111111111111111111111111111111111111";

    let token_id = U256::from(112_u64);
    let builder_code = B256::from_str(BUILDER_CODE).expect("builder code");
    let config = Config::builder()
        .builder(
            BuilderConfig::builder()
                .builder_code(BUILDER_CODE.to_owned())
                .build(),
        )
        .build();
    let client = configured_client(token_id, TickSize::Hundredth, config).await;

    let signable = client
        .limit_order()
        .token_id(token_id)
        .price(dec("0.55"))
        .size(dec("10"))
        .side(Side::Buy)
        .build()
        .await
        .expect("signable order");

    assert_eq!(signable.order.builder, builder_code);
}
