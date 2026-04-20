#![allow(
    clippy::print_stdout,
    reason = "Examples intentionally print user-visible output"
)]

use std::env;
use std::io::Error as IoError;
use std::str::FromStr as _;

use alloy::primitives::U256;
use polymarket_client_sdk::clob::types::Side;
use polymarket_client_sdk::clob::{Client, Config};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let host = env::var("POLYMARKET_CLOB_HOST")
        .unwrap_or_else(|_| "https://clob.polymarket.com".to_owned());
    let token_id = env::var("POLYMARKET_TOKEN_ID").map_err(|error| {
        IoError::other(format!(
            "set POLYMARKET_TOKEN_ID to a live CLOB token id before running this example: {error}"
        ))
    })?;
    let token_id = U256::from_str(&token_id)?;
    let client = Client::new(&host, Config::default())?;

    println!("midpoint: {:?}", client.midpoint(token_id).await?);
    println!("price: {:?}", client.price(token_id, Side::Buy).await?);
    println!("spread: {:?}", client.spread(token_id).await?);

    Ok(())
}
