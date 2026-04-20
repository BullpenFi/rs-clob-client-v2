# polymarket-client-sdk

Bullpen.fi's best-guess Rust implementation of Polymarket's CLOB V2 SDK.

This repository is maintained by Bullpen.fi. It is not an official Polymarket release. The implementation is intentionally shaped to match Polymarket's official Rust V1 SDK distribution and public import surface as closely as practical, while tracking the TypeScript V2 client behavior.

This crate is meant for testing and early Rust adoption of Polymarket CLOB V2. If Polymarket releases an official Rust CLOB V2 SDK, users should switch to the official SDK rather than rely on this repository long term.

## Distribution strategy

- Package name: `polymarket-client-sdk`
- Rust import path: `polymarket_client_sdk`
- Repository: `BullpenFi/rs-clob-client-v2`
- Publish policy: `publish = false`

The package name intentionally matches Polymarket's official Rust SDK so downstream code can use the same dependency key and import path during testing. This repo is not published to crates.io under that name; use it as a git or path dependency only.

## Status

- Unofficial Bullpen.fi implementation
- Best-guess V2 surface and behavior based on Polymarket's TypeScript V2 SDK
- V1-style Rust architecture and packaging where practical
- V2-only client surface
- Type-state client with authenticated and unauthenticated modes
- `alloy`-based L1 auth and V2 EIP-712 order signing
- Authenticated and public REST endpoints
- V2 limit and market order builders
- Feature-gated WebSocket scaffolding behind `ws`

## Getting started

Use this repository as a git dependency:

```toml
[dependencies]
polymarket-client-sdk = { git = "https://github.com/BullpenFi/rs-clob-client-v2", branch = "main" }
```

When Polymarket releases an official Rust V2 SDK, the intended migration should be mostly a dependency source change rather than a crate-path rewrite.

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
use polymarket_client_sdk::auth::PrivateKeySigner;
use polymarket_client_sdk::clob::{Client, Config};
use polymarket_client_sdk::POLYGON;

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
use polymarket_client_sdk::auth::PrivateKeySigner;
use polymarket_client_sdk::clob::types::Side;
use polymarket_client_sdk::clob::{Client, Config, UserOrder};
use polymarket_client_sdk::types::Decimal;
use polymarket_client_sdk::POLYGON;

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
