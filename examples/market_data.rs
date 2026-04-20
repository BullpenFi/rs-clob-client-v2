#![allow(
    clippy::print_stdout,
    reason = "Examples intentionally print user-visible output"
)]

use alloy::primitives::U256;
use polymarket_client_sdk::clob::types::Side;
use polymarket_client_sdk::clob::{Client, Config};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new("https://clob.polymarket.com", Config::default())?;
    let token_id = U256::from(1_u64);

    println!("midpoint: {:?}", client.midpoint(token_id).await?);
    println!("price: {:?}", client.price(token_id, Side::Buy).await?);
    println!("spread: {:?}", client.spread(token_id).await?);

    Ok(())
}
