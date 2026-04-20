use alloy::primitives::U256;
use polymarket_clob_client_v2::clob::{Client, Config};
use polymarket_clob_client_v2::clob::types::Side;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new("https://clob.polymarket.com", Config::default())?;
    let token_id = U256::from(1_u64);

    println!("midpoint: {:?}", client.midpoint(token_id).await?);
    println!("price: {:?}", client.price(token_id, Side::Buy).await?);
    println!("spread: {:?}", client.spread(token_id).await?);

    Ok(())
}
