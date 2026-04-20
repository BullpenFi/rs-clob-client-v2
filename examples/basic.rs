#![allow(
    clippy::print_stdout,
    reason = "Examples intentionally print user-visible output"
)]

use polymarket_clob_client_v2::clob::{Client, Config};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new("https://clob.polymarket.com", Config::default())?;
    println!("ok: {}", client.ok().await?);
    println!("server time: {}", client.server_time().await?);
    Ok(())
}
