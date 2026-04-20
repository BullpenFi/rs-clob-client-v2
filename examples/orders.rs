use std::str::FromStr;

use alloy::primitives::U256;
use alloy::signers::Signer as _;
use polymarket_clob_client_v2::auth::PrivateKeySigner;
use polymarket_clob_client_v2::clob::{Client, Config, UserOrder};
use polymarket_clob_client_v2::clob::types::{OrderType, Side};
use polymarket_clob_client_v2::types::Decimal;
use polymarket_clob_client_v2::POLYGON;

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
