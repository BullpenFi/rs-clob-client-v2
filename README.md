# rs-clob-client-v2

Rust rewrite of Polymarket's `@polymarket/clob-client-v2`, following the architecture and idioms of Polymarket's official Rust V1 SDK.

## Status

- V2-only client surface
- Type-state client with authenticated and unauthenticated modes
- `alloy`-based L1 auth and V2 EIP-712 order signing
- Authenticated and public REST endpoints
- V2 limit and market order builders
- Feature-gated WebSocket scaffolding behind `ws`

## Crate layout

- `src/clob/client.rs`: type-state HTTP client
- `src/clob/order_builder.rs`: V2 limit/market order construction
- `src/clob/types/`: request, response, market, trade, order, and enum models
- `src/auth.rs`: L1/L2/builder auth helpers
- `src/config.rs`: chain to contract configuration

## Quick start

```rust,no_run
use std::str::FromStr;

use alloy::signers::Signer as _;
use polymarket_clob_client_v2::auth::PrivateKeySigner;
use polymarket_clob_client_v2::clob::{Client, Config};
use polymarket_clob_client_v2::POLYGON;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let signer = PrivateKeySigner::from_str("0x...")?.with_chain_id(Some(POLYGON));

    let client = Client::new("https://clob.polymarket.com", Config::default())?;
    let auth = client.authentication_builder(&signer).authenticate().await?;

    println!("server time: {}", auth.server_time().await?);
    println!("ok: {}", auth.ok().await?);

    Ok(())
}
```

## Order flow

```rust,no_run
use std::str::FromStr;

use alloy::primitives::U256;
use alloy::signers::Signer as _;
use polymarket_clob_client_v2::auth::PrivateKeySigner;
use polymarket_clob_client_v2::clob::{Client, Config, UserOrder};
use polymarket_clob_client_v2::clob::types::Side;
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

    let signed = client.create_order(&signer, order).await?;
    let response = client.post_order(&signed).await?;
    println!("{response:?}");

    Ok(())
}
```

## Features

- Default: REST client and V2 order builder
- `ws`: generic WebSocket transport and CLOB subscription scaffolding
- `tracing`: reserved for trace instrumentation
