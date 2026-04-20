#![allow(
    clippy::print_stdout,
    reason = "Examples intentionally print user-visible output"
)]

use std::str::FromStr as _;

use alloy::primitives::U256;
use alloy::signers::Signer as _;
use polymarket_client_sdk::POLYGON;
use polymarket_client_sdk::auth::PrivateKeySigner;
use polymarket_client_sdk::clob::types::{OrderType, Side};
use polymarket_client_sdk::clob::{Client, Config, UserOrder};
use polymarket_client_sdk::types::Decimal;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let signer = PrivateKeySigner::from_str("0x...")?.with_chain_id(Some(POLYGON));
    let client = Client::new("https://clob.polymarket.com", Config::default())?
        .authentication_builder(&signer)
        .authenticate()
        .await?;

    let order = UserOrder::builder()
        .token_id(U256::from(1_u64))
        .price(Decimal::from_str("0.55")?)
        .size(Decimal::from_str("10.00")?)
        .side(Side::Buy)
        .build();

    let response = client
        .create_and_post_order(&signer, order, OrderType::Gtc, false, false)
        .await?;

    println!("{response:?}");
    Ok(())
}
