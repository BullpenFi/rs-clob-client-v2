# Integration Guide

This repository is Bullpen.fi's unofficial, best-guess Rust implementation of Polymarket CLOB V2.
It is intended for downstream testing and early integration work, not as a permanent substitute for an eventual official Polymarket Rust V2 SDK.

## Recommended dependency setup

Use a pinned git revision, not `branch = "main"`, in downstream applications.

```toml
[dependencies]
polymarket-client-sdk = { git = "https://github.com/BullpenFi/rs-clob-client-v2", rev = "fbe9c45729fd5d883b7fdec3fe2ddbbabbc44190" }
```

Why this shape:
- package name matches Polymarket's official Rust SDK surface: `polymarket-client-sdk`
- import path matches the official-style surface: `polymarket_client_sdk`
- pinning `rev` avoids silent behavior drift during integration testing
- later migration to an official Rust V2 SDK should mostly be a dependency-source change

## Import path

Consumer code should import this crate exactly as it would import an eventual official SDK:

```rust
use polymarket_client_sdk::clob::Client;
```

## Intended migration path

The intended future cutover is:

```toml
# Current testing setup
[dependencies]
polymarket-client-sdk = { git = "https://github.com/BullpenFi/rs-clob-client-v2", rev = "fbe9c45729fd5d883b7fdec3fe2ddbbabbc44190" }

# Future official setup once Polymarket ships a Rust V2 crate
[dependencies]
polymarket-client-sdk = "<official-version>"
```

If your code imports `polymarket_client_sdk::...`, the migration should not require a crate-path rewrite.

## Feature flags

Default integration surface:
- default features include `clob`

Optional features:
- `ws`: websocket transport and CLOB subscription scaffolding
- `tracing`: deserialization warnings and request instrumentation support

Example:

```toml
[dependencies]
polymarket-client-sdk = {
  git = "https://github.com/BullpenFi/rs-clob-client-v2",
  rev = "fbe9c45729fd5d883b7fdec3fe2ddbbabbc44190",
  features = ["ws", "tracing"]
}
```

## Basic integration patterns

### 1. Public read-only client

```rust,no_run
use polymarket_client_sdk::clob::{Client, Config};

#[tokio::main]
async fn main() -> polymarket_client_sdk::Result<()> {
    let client = Client::new("https://clob.polymarket.com", Config::default())?;

    println!("ok: {}", client.ok().await?);
    println!("server time: {}", client.server_time().await?);

    Ok(())
}
```

### 2. Authenticated client

```rust,no_run
use std::str::FromStr;

use alloy::signers::Signer as _;
use polymarket_client_sdk::POLYGON;
use polymarket_client_sdk::auth::PrivateKeySigner;
use polymarket_client_sdk::clob::{Client, Config};

#[tokio::main]
async fn main() -> polymarket_client_sdk::Result<()> {
    let signer = PrivateKeySigner::from_str("0x...")?.with_chain_id(Some(POLYGON));
    let client = Client::new("https://clob.polymarket.com", Config::default())?
        .authentication_builder(&signer)
        .authenticate()
        .await?;

    println!("api keys: {:?}", client.api_keys().await?);
    Ok(())
}
```

### 3. Order creation

```rust,no_run
use std::str::FromStr;

use alloy::primitives::U256;
use alloy::signers::Signer as _;
use polymarket_client_sdk::POLYGON;
use polymarket_client_sdk::auth::PrivateKeySigner;
use polymarket_client_sdk::clob::types::Side;
use polymarket_client_sdk::clob::{Client, Config, UserOrder};
use polymarket_client_sdk::types::Decimal;

#[tokio::main]
async fn main() -> polymarket_client_sdk::Result<()> {
    let signer = PrivateKeySigner::from_str("0x...")?.with_chain_id(Some(POLYGON));
    let client = Client::new("https://clob.polymarket.com", Config::default())?
        .authentication_builder(&signer)
        .authenticate()
        .await?;

    let order = UserOrder::builder()
        .token_id(U256::from_str("<token-id>")?)
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

## Integration guardrails

Use this crate for:
- downstream integration testing
- internal tools and experiments
- pre-official SDK migration prep
- validating Polymarket CLOB V2 workflows in Rust

Do not treat this crate as:
- an official Polymarket SDK
- a crates.io release candidate
- a permanent dependency contract if Polymarket later ships an official V2 crate

## Downstream smoke checklist

Before integrating into a larger app, validate these in the downstream environment:

1. Dependency resolution
   - `cargo check`
2. Public HTTP reachability
   - instantiate `Client::new("https://clob.polymarket.com", Config::default())`
   - call `ok()`
   - call `server_time()`
3. Public market-data reads
   - call `midpoint(token_id)`
   - call `price(token_id, Side::Buy)`
   - call `spread(token_id)`
4. Auth flow, if credentials are available
   - authenticate with a signer
   - call `api_keys()` or `balance_allowance()`
5. Order pipeline, if credentials are available
   - build one order
   - sign it
   - only post to a real environment if that is explicitly intended for the test account
6. Optional websocket flow
   - enable the `ws` feature
   - subscribe and confirm event decoding in your runtime environment

## Example commands

Public smoke commands from this repo:

```bash
cargo run --example basic
POLYMARKET_TOKEN_ID=<live-token-id> cargo run --example market_data
```

Authenticated example from this repo requires a real signer and token id:

```bash
POLYMARKET_PRIVATE_KEY=0x... \
POLYMARKET_TOKEN_ID=<live-token-id> \
  cargo run --example orders
```

A live token id can be obtained from Polymarket's public market metadata. One practical source is the Gamma API's `clobTokenIds` field.

## Current known-good baseline

- Repository commit used for this integration guidance: `fbe9c45729fd5d883b7fdec3fe2ddbbabbc44190`
- Local verification at that baseline:
  - `cargo check`
  - `cargo test`
  - `cargo test --features ws`
  - `cargo clippy --all-targets --all-features -- -D warnings`
  - `RUSTDOCFLAGS='-D warnings' cargo doc --no-deps`
- GitHub Actions at that baseline:
  - `CI`: green
  - `Release-plz`: green

## Latest live public smoke result

Executed on 2026-04-20 against real Polymarket endpoints:

- `cargo run --example basic`
  - passed
  - verified `ok()` and `server_time()` against `https://clob.polymarket.com`
- `POLYMARKET_TOKEN_ID=8501497159083948713316135768103773293754490207922884688769443031624417212426 cargo run --example market_data`
  - passed
  - verified `midpoint()`, `price()`, and `spread()` against a live public token
- Authenticated smoke using `cargo run --example orders`
  - not executed in this environment
  - blocked only by missing `POLYMARKET_PRIVATE_KEY`
