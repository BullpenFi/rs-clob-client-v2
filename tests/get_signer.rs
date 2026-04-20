mod common;

use std::str::FromStr as _;

use alloy::primitives::U256;
use alloy::signers::Signer as _;
use polymarket_clob_client_v2::auth::PrivateKeySigner;
use polymarket_clob_client_v2::clob::{Config, UserOrder};
use polymarket_clob_client_v2::clob::types::Side;
use polymarket_clob_client_v2::config::exchange_contract;
use polymarket_clob_client_v2::types::Decimal;
use polymarket_clob_client_v2::{Error, POLYGON};

fn alternate_signer() -> PrivateKeySigner {
    PrivateKeySigner::from_str(
        "0x8b3a350cf5c34c9194ca3a545d4df2f3b0c5adce3b067a729b9f3cf0f4a8e15d",
    )
    .expect("valid private key")
    .with_chain_id(Some(POLYGON))
}

async fn configured_client(config: Config) -> polymarket_clob_client_v2::clob::Client<
    polymarket_clob_client_v2::auth::state::Authenticated<polymarket_clob_client_v2::auth::Normal>,
> {
    let client = common::create_authenticated("https://clob.polymarket.com", config).await;
    client.set_tick_size(
        U256::from(1_u64),
        polymarket_clob_client_v2::clob::types::TickSize::Hundredth,
    );
    client.set_neg_risk(U256::from(1_u64), false);
    client
}

fn sample_user_order() -> UserOrder {
    UserOrder::builder()
        .token_id(U256::from(1_u64))
        .price(Decimal::from_str("0.55").expect("decimal"))
        .size(Decimal::from_str("10").expect("decimal"))
        .side(Side::Buy)
        .build()
}

#[tokio::test]
async fn static_signer_fallback_signs_orders_without_factory() {
    let signer = common::signer();
    let client = configured_client(Config::default()).await;

    let signed = client
        .create_order(&signer, sample_user_order())
        .await
        .expect("signed order");

    let verifying_contract = exchange_contract(POLYGON, false).expect("exchange contract");
    let hash = polymarket_clob_client_v2::clob::types::signing_hash(
        &signed.order,
        POLYGON,
        verifying_contract,
    );

    assert_eq!(signed.order.signer, signer.address());
    assert_eq!(
        signed
            .signature
            .recover_address_from_prehash(&hash)
            .expect("recover signer"),
        signer.address()
    );
}

#[tokio::test]
async fn order_builder_get_signer_uses_factory_signer_for_build_and_sign() {
    let client = configured_client(Config::default()).await;
    let static_signer = common::signer();
    let dynamic_signer = alternate_signer();

    let builder = client
        .limit_order()
        .token_id(U256::from(1_u64))
        .price(Decimal::from_str("0.55").expect("decimal"))
        .size(Decimal::from_str("10").expect("decimal"))
        .side(Side::Buy)
        .get_signer({
            let dynamic_signer = dynamic_signer.clone();
            move || {
                let dynamic_signer = dynamic_signer.clone();
                async move {
                    Ok(
                        Box::new(dynamic_signer)
                            as Box<dyn polymarket_clob_client_v2::auth::Signer + Send + Sync>,
                    )
                }
            }
        });

    let signable = builder.build().await.expect("signable order");
    let signed = client
        .sign(&dynamic_signer, signable)
        .await
        .expect("signed order");
    let verifying_contract = exchange_contract(POLYGON, false).expect("exchange contract");
    let hash = polymarket_clob_client_v2::clob::types::signing_hash(
        &signed.order,
        POLYGON,
        verifying_contract,
    );

    assert_ne!(signed.order.signer, static_signer.address());
    assert_eq!(signed.order.signer, dynamic_signer.address());
    assert_eq!(
        signed
            .signature
            .recover_address_from_prehash(&hash)
            .expect("recover signer"),
        dynamic_signer.address()
    );
}

#[tokio::test]
async fn config_get_signer_takes_precedence_over_static_signer() {
    let static_signer = common::signer();
    let dynamic_signer = alternate_signer();
    let client = configured_client(Config::default().get_signer({
        let dynamic_signer = dynamic_signer.clone();
        move || {
            let dynamic_signer = dynamic_signer.clone();
            async move {
                Ok(
                    Box::new(dynamic_signer)
                        as Box<dyn polymarket_clob_client_v2::auth::Signer + Send + Sync>,
                )
            }
        }
    }))
    .await;

    let signed = client
        .create_order(&static_signer, sample_user_order())
        .await
        .expect("signed order");
    let verifying_contract = exchange_contract(POLYGON, false).expect("exchange contract");
    let hash = polymarket_clob_client_v2::clob::types::signing_hash(
        &signed.order,
        POLYGON,
        verifying_contract,
    );

    assert_eq!(signed.order.signer, dynamic_signer.address());
    assert_eq!(
        signed
            .signature
            .recover_address_from_prehash(&hash)
            .expect("recover signer"),
        dynamic_signer.address()
    );
}

#[tokio::test]
async fn get_signer_factory_error_propagates() {
    let client = configured_client(Config::default().get_signer(|| async {
        Err(Error::validation("dynamic signer failure"))
    }))
    .await;

    let error = client
        .create_order(&common::signer(), sample_user_order())
        .await
        .expect_err("factory error should propagate");

    assert!(error.to_string().contains("dynamic signer failure"));
}
