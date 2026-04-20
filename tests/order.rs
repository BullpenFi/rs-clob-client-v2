mod common;

use std::str::FromStr as _;

use alloy::primitives::U256;
use httpmock::Method::GET;
use httpmock::MockServer;
use polymarket_clob_client_v2::clob::types::{BuilderConfig, FeeInfo, OrderType, Side, TickSize};
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
async fn market_order_builder_rejects_prices_outside_tick_bounds() {
    let client = common::create_authenticated(common::TEST_HOST, Config::default()).await;
    let token_id = U256::from(125_u64);
    client.set_tick_size(token_id, TickSize::Hundredth);
    client.set_neg_risk(token_id, false);

    for price in ["0.009", "0.991"] {
        let error = client
            .market_order()
            .token_id(token_id)
            .price(Decimal::from_str(price).expect("decimal"))
            .amount(Decimal::from_str("10").expect("decimal"))
            .side(Side::Buy)
            .build()
            .await
            .expect_err("out-of-range price should fail");

        assert!(
            error
                .to_string()
                .contains("price 0.009 must be between 0.01 and 0.99")
                || error
                    .to_string()
                    .contains("price 0.991 must be between 0.01 and 0.99")
        );
    }
}

#[tokio::test]
async fn market_orders_reject_unsupported_order_types() {
    let signer = common::signer();
    let server = MockServer::start();
    let version = server.mock(|when, then| {
        when.method(GET).path("/version");
        then.status(200)
            .json_body_obj(&serde_json::json!({ "version": 2 }));
    });

    let client = common::create_authenticated(
        &server.base_url(),
        Config::builder().allow_insecure(true).build(),
    )
    .await;
    let token_id = U256::from(126_u64);
    client.set_tick_size(token_id, TickSize::Hundredth);
    client.set_neg_risk(token_id, false);

    for order_type in [OrderType::Gtc, OrderType::Gtd] {
        let builder_error = client
            .market_order()
            .token_id(token_id)
            .price(Decimal::from_str("0.55").expect("decimal"))
            .amount(Decimal::from_str("10").expect("decimal"))
            .side(Side::Buy)
            .order_type(order_type)
            .build()
            .await
            .expect_err("builder should reject unsupported market order type");
        let create_error = client
            .create_market_order(
                &signer,
                UserMarketOrder::builder()
                    .token_id(token_id)
                    .price(Decimal::from_str("0.55").expect("decimal"))
                    .amount(Decimal::from_str("10").expect("decimal"))
                    .side(Side::Buy)
                    .order_type(order_type)
                    .build(),
            )
            .await
            .expect_err("create_market_order should reject unsupported market order type");
        let post_error = client
            .create_and_post_market_order(
                &signer,
                UserMarketOrder::builder()
                    .token_id(token_id)
                    .price(Decimal::from_str("0.55").expect("decimal"))
                    .amount(Decimal::from_str("10").expect("decimal"))
                    .side(Side::Buy)
                    .build(),
                order_type,
                false,
            )
            .await
            .expect_err("create_and_post_market_order should reject unsupported market order type");

        for error in [builder_error, create_error, post_error] {
            assert!(
                error
                    .to_string()
                    .contains("market orders only support FOK and FAK order types"),
                "unexpected error: {error}"
            );
        }
    }

    version.assert_calls(1);
}

#[tokio::test]
async fn create_market_order_treats_explicit_zero_price_as_market_price() {
    let server = MockServer::start();
    let book = server.mock(|when, then| {
        when.method(GET)
            .path("/book")
            .query_param("token_id", "127");
        then.status(200).json_body_obj(&serde_json::json!({
            "market": "market-1",
            "asset_id": "127",
            "timestamp": "1700000000",
            "hash": "",
            "min_order_size": "1",
            "tick_size": "0.01",
            "neg_risk": false,
            "bids": [],
            "asks": [
                { "price": "0.50", "size": "20" },
                { "price": "0.45", "size": "20" }
            ]
        }));
    });

    let signer = common::signer();
    let client = common::create_authenticated(
        &server.base_url(),
        Config::builder().allow_insecure(true).build(),
    )
    .await;
    let token_id = U256::from(127_u64);
    client.set_tick_size(token_id, TickSize::Hundredth);
    client.set_neg_risk(token_id, false);

    let signed = client
        .create_market_order(
            &signer,
            UserMarketOrder::builder()
                .token_id(token_id)
                .price(Decimal::ZERO)
                .amount(Decimal::from_str("9").expect("decimal"))
                .side(Side::Buy)
                .build(),
        )
        .await
        .expect("zero price should fall back to market price");

    book.assert();
    assert_eq!(signed.order.makerAmount.to_string(), "9000000");
    assert_eq!(signed.order.takerAmount.to_string(), "20000000");
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
