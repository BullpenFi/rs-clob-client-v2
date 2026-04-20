#![allow(
    clippy::print_stdout,
    reason = "Examples intentionally print user-visible output"
)]

use std::env;
use std::io::Error as IoError;
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
    let host = env::var("POLYMARKET_CLOB_HOST")
        .unwrap_or_else(|_| "https://clob.polymarket.com".to_owned());
    let private_key = env::var("POLYMARKET_PRIVATE_KEY").map_err(|error| {
        IoError::other(format!(
            "set POLYMARKET_PRIVATE_KEY before running the authenticated orders example: {error}"
        ))
    })?;
    let token_id = env::var("POLYMARKET_TOKEN_ID").map_err(|error| {
        IoError::other(format!(
            "set POLYMARKET_TOKEN_ID to a live CLOB token id before running this example: {error}"
        ))
    })?;

    let signer = PrivateKeySigner::from_str(&private_key)?.with_chain_id(Some(POLYGON));
    let client = Client::new(&host, Config::default())?
        .authentication_builder(&signer)
        .authenticate()
        .await?;

    let order = UserOrder::builder()
        .token_id(U256::from_str(&token_id)?)
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
