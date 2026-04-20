# Plan 0001: rs-clob-client-v2 Architecture & Implementation

**Status:** Draft
**Created:** 2026-04-19
**Goal:** Rewrite the TypeScript `@polymarket/clob-client-v2` in Rust, following the patterns established by Polymarket's official Rust V1 SDK (`rs-clob-client`), so it can serve as a drop-in replacement when Polymarket releases their own Rust V2 client.

---

## 1. Context & Strategy

### What we're building
A Rust crate (`polymarket-clob-client-v2` or similar) that mirrors the TypeScript V2 SDK's full API surface while adopting the idiomatic Rust patterns from Polymarket's V1 Rust SDK.

### Design principles
1. **Mirror V1 Rust architecture** — type-state client, sealed traits, `bon` builders, `alloy` crypto, feature-gated modules, `Arc<Inner>` sharing.
2. **Match V2 TypeScript API surface** — every public method in the TS client should have a Rust equivalent.
3. **Drop-in replacement ready** — naming, types, and module structure should be close enough that migrating to an official Polymarket V2 Rust SDK would be straightforward.
4. **Incremental buildability** — each phase should compile and be independently testable.

### Key V1→V2 differences (from TS SDK analysis)
| Aspect | V1 | V2 |
|---|---|---|
| Order fields | `nonce`, `feeRateBps`, `taker` | `timestamp`, `metadata`, `builder`, `expiration` (unix s) |
| Signature types | EOA, Proxy, GnosisSafe | + `POLY_1271` (EIP-1271 smart contract wallets) |
| EIP-712 domain version | `"1"` | `"2"` |
| Exchange contracts | `exchange`, `negRiskExchange` | `exchangeV2`, `negRiskExchangeV2` |
| Fee handling | User-specified `feeRateBps` | Platform-calculated server-side |
| Builder support | None | Full (`builderCode`, builder API keys, builder trades) |
| Order scoring | None | `isOrderScoring`, `areOrdersScoring` |
| Taker field | Explicit (`address(0)` = public) | Removed from order struct |
| Nonce field | Explicit (onchain cancel) | Removed |

---

## 2. Module Structure

```
rs-clob-client-v2/
├── Cargo.toml
├── src/
│   ├── lib.rs                    # Crate root, re-exports, wallet derivation
│   ├── error.rs                  # Error types (Kind enum, Error struct w/ backtrace)
│   ├── types.rs                  # Re-exported external types (Address, U256, etc.)
│   ├── serde_helpers.rs          # StringFromAny, flexible deser helpers
│   ├── config.rs                 # Contract addresses per chain (phf map)
│   ├── auth.rs                   # L1 (EIP-712), L2 (HMAC), Builder auth
│   ├── clob/
│   │   ├── mod.rs
│   │   ├── client.rs             # Client<S: State> with type-state pattern
│   │   ├── order_builder.rs      # OrderBuilder<Limit/Market, K> (V2 fields)
│   │   ├── types/
│   │   │   ├── mod.rs
│   │   │   ├── order.rs          # OrderV2 (EIP-712 struct), SignedOrderV2
│   │   │   ├── market.rs         # Market, MarketDetails, ClobToken
│   │   │   ├── trade.rs          # Trade, MakerOrder
│   │   │   ├── book.rs           # OrderBookSummary, OrderSummary
│   │   │   ├── request.rs        # All request param types
│   │   │   ├── response.rs       # All response types
│   │   │   ├── enums.rs          # Side, OrderType, SignatureTypeV2, Chain, etc.
│   │   │   └── builder.rs        # BuilderConfig, builder-related types
│   │   └── ws/                   # WebSocket support (feature-gated)
│   │       ├── mod.rs
│   │       ├── orderbook.rs
│   │       └── user.rs
│   └── ws/                       # Generic WS infrastructure (from V1)
│       ├── mod.rs
│       ├── connection.rs
│       ├── config.rs
│       └── traits.rs
├── tests/
│   ├── common/mod.rs             # Shared test utilities
│   ├── auth.rs
│   ├── client.rs
│   └── order.rs
├── examples/
│   ├── basic.rs
│   ├── orders.rs
│   └── market_data.rs
└── benches/
    └── order_operations.rs
```

---

## 3. Implementation Phases

### Phase 1: Foundation (Compiles, no network)
**Files:** `Cargo.toml`, `lib.rs`, `error.rs`, `types.rs`, `config.rs`, `serde_helpers.rs`

- [ ] Set up `Cargo.toml` with dependencies mirroring V1 (`alloy`, `reqwest`, `serde`, `bon`, `tokio`, `rust_decimal`, `chrono`, `secrecy`, `hmac`, `sha2`, `dashmap`, `phf`, `base64`, `rand`, `url`)
- [ ] Implement `error.rs` — `Kind` enum (Status, Validation, Synchronization, Internal, WebSocket, Geoblock), `Error` struct with backtrace, `Result<T>` alias, `From` impls
- [ ] Implement `types.rs` — re-export `Address`, `U256`, `Decimal`, `Signature`, `DateTime`
- [ ] Implement `config.rs` — V2 contract addresses for Polygon (137) and Amoy (80002) using `phf_map!`
  - `exchangeV2` and `negRiskExchangeV2` addresses (from TS `config.ts`)
- [ ] Implement `serde_helpers.rs` — `StringFromAny` and flexible deserialization (from V1)
- [ ] Implement `clob/types/enums.rs` — `Chain`, `Side`, `OrderType`, `SignatureTypeV2` (with `POLY_1271`), `TickSize`, `AssetType`, `PriceHistoryInterval`

**Verification:** `cargo build` succeeds, `cargo test` runs (unit tests for enums, config lookups, error creation).

### Phase 2: Authentication & Signing
**Files:** `auth.rs`, `clob/types/order.rs`

- [ ] Implement L1 auth (EIP-712 `ClobAuth` domain signing) — same as V1 since auth mechanism unchanged
- [ ] Implement L2 auth (HMAC-SHA256) — same as V1
- [ ] Implement Builder auth — same as V1
- [ ] Define `OrderV2` EIP-712 struct:
  ```rust
  sol! {
      struct Order {
          uint256 salt;
          address maker;
          address signer;
          uint256 tokenId;
          uint256 makerAmount;
          uint256 takerAmount;
          uint8 side;
          uint8 signatureType;
          uint256 timestamp;    // NEW in V2
          bytes32 metadata;     // NEW in V2
          bytes32 builder;      // NEW in V2
          uint256 expiration;   // Changed semantics in V2
      }
  }
  ```
- [ ] EIP-712 domain: `name = "Polymarket CTF Exchange"`, `version = "2"`, `chainId`
- [ ] Implement `SignatureTypeV2` with `POLY_1271 = 3` variant
- [ ] Implement order signing using `alloy::signers::Signer`

**Verification:** Unit tests for L1/L2 header generation, order struct hashing, signature round-trip.

### Phase 3: HTTP Client & Core Types
**Files:** `clob/client.rs`, `clob/types/{market,trade,book,request,response,builder}.rs`

- [ ] Define all request/response types matching the TS SDK (with `#[derive(Builder, Serialize, Deserialize)]`)
- [ ] Implement `Client<S: State>` skeleton with type-state pattern:
  - `Unauthenticated` — public endpoints only
  - `Authenticated<K: Kind>` — `Normal` or `Builder`
- [ ] Implement `AuthenticationBuilder` for credential setup + state transition
- [ ] Implement the HTTP request handler (reqwest wrapper with auth header injection)
- [ ] Implement retry logic (transient errors, 5xx, network errors, 30ms delay)
- [ ] Internal caches with `DashMap` for tick_size, neg_risk

**Verification:** `cargo build` succeeds with all types defined. Unit tests for type serialization.

### Phase 4: Public Endpoints (Read-Only)
**Files:** `clob/client.rs` (add methods)

- [ ] Health: `ok()`, `server_time()`
- [ ] Markets: `markets()`, `market()`, `simplified_markets()`, `sampling_markets()`, `sampling_simplified_markets()`
- [ ] Pricing: `midpoint()`, `midpoints()`, `price()`, `prices()`, `spread()`, `spreads()`, `last_trade_price()`, `last_trades_prices()`, `prices_history()`
- [ ] Order book: `order_book()`, `order_books()`, `order_book_hash()`
- [ ] Market config: `tick_size()`, `neg_risk()`, `fee_exponent()`
- [ ] Pagination: `stream_data()` generic pagination helper
- [ ] Market price calculation: `calculate_market_price()`
- [ ] Market trades events: `market_trades_events()`

**Verification:** Integration tests with `httpmock` for each endpoint group.

### Phase 5: Authenticated Endpoints
**Files:** `clob/client.rs` (add methods), `clob/order_builder.rs`

- [ ] API key management: `create_api_key()`, `derive_api_key()`, `create_or_derive_api_key()`, `api_keys()`, `delete_api_key()`
- [ ] Read-only API keys: `create_readonly_api_key()`, `readonly_api_keys()`, `delete_readonly_api_key()`
- [ ] Orders: `order()`, `orders()` (open orders with pagination)
- [ ] Trades: `trades()`, `trades_paginated()`
- [ ] Balance: `balance_allowance()`, `update_balance_allowance()`
- [ ] Notifications: `notifications()`, `drop_notifications()`
- [ ] Account: `closed_only_mode()`
- [ ] Heartbeat: `heartbeat()`

**Verification:** Integration tests with mock server for authenticated endpoints.

### Phase 6: Order Builder & Management
**Files:** `clob/order_builder.rs`, `clob/client.rs`

- [ ] `OrderBuilder<Limit, K>` — V2 fields (no nonce/feeRateBps/taker, add timestamp/metadata/builder)
  - Validation: price range, tick size, lot size
  - Amount calculation (maker/taker amounts)
  - Salt generation
- [ ] `OrderBuilder<Market, K>` — auto-price from orderbook, amount types (USDC/Shares)
  - `calculate_buy_market_price()`, `calculate_sell_market_price()`
- [ ] Client methods:
  - `limit_order()` → `OrderBuilder<Limit>`
  - `market_order()` → `OrderBuilder<Market>`
  - `sign()` — sign order with EIP-712
  - `post_order()`, `post_orders()` — submit signed orders
  - `cancel_order()`, `cancel_orders()`, `cancel_all_orders()`, `cancel_market_orders()`
- [ ] Order type support: GTC, GTD (with expiration), FOK, FAK, post_only, defer_exec

**Verification:** Unit tests for amount calculations, order validation. Integration tests for order submission/cancellation.

### Phase 7: Builder & Rewards APIs
**Files:** `clob/client.rs`, `clob/types/builder.rs`

- [ ] Builder API keys: `create_builder_api_key()`, `builder_api_keys()`, `revoke_builder_api_key()`
- [ ] Builder trades: `builder_trades()`
- [ ] Order scoring: `order_scoring()`, `orders_scoring()`
- [ ] Rewards: `current_rewards()`, `reward_percentages()`, `earnings_for_user_for_day()`, `total_earnings_for_user_for_day()`, `user_earnings_and_markets_config()`, `raw_rewards_for_market()`

**Verification:** Integration tests with mock server.

### Phase 8: WebSocket Support (Feature-Gated)
**Files:** `ws/`, `clob/ws/`

- [ ] Generic WS infrastructure (connection manager, reconnection, config)
- [ ] CLOB-specific WS: orderbook subscriptions, user stream
- [ ] Message types: `BookUpdate`, `OrderMessage`, `TradeMessage`, `MidpointUpdate`
- [ ] Feature gate: `#[cfg(feature = "ws")]`

**Verification:** Integration tests (may require live endpoint or mock WS server).

### Phase 9: Examples, Docs, Polish
- [ ] Examples: `basic.rs`, `orders.rs`, `market_data.rs`, `builder.rs`
- [ ] API documentation (`///` doc comments on all public items)
- [ ] README with usage, authentication guide, migration notes
- [ ] Benchmarks for order building and serialization
- [ ] CI setup (GitHub Actions for `cargo test`, `cargo clippy`, `cargo fmt`)

---

## 4. Dependency Matrix

```toml
[dependencies]
# Async runtime
tokio = { version = "1", features = ["full"] }

# HTTP
reqwest = { version = "0.13", default-features = false, features = ["json", "query", "rustls"] }

# Crypto (matching V1's alloy usage)
alloy = { version = "1.6", features = ["signers", "signer-local", "sol-types", "eips"] }

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"
serde_with = "3"
serde_repr = "0.1"
serde_html_form = "0.2"

# Types
rust_decimal = { version = "1", features = ["serde-with-str"] }
chrono = { version = "0.4", features = ["serde"] }
url = "2"
uuid = { version = "1", features = ["v4"] }

# Auth
hmac = "0.12"
sha2 = "0.10"
base64 = "0.22"
secrecy = "0.10"

# Builders
bon = "3"

# Concurrent data
dashmap = "6"
phf = { version = "0.11", features = ["macros"] }

# Utilities
rand = "0.9"
strum_macros = "0.27"
bitflags = "2"
futures = "0.3"
async-trait = "0.1"

# Error handling
backoff = { version = "0.4", features = ["tokio"] }

# Optional
tokio-tungstenite = { version = "0.29", optional = true }

[features]
default = ["clob"]
clob = []
ws = ["dep:tokio-tungstenite"]
tracing = ["dep:tracing"]
```

*Note: Exact versions should be verified against current crates.io at implementation time.*

---

## 5. Type Mapping Reference (TS → Rust)

| TypeScript | Rust |
|---|---|
| `ClobClient` | `Client<S: State>` |
| `ClobSigner` (viem/ethers) | `impl alloy::signers::Signer` |
| `ApiKeyCreds` | `Credentials` (with `secrecy::Secret`) |
| `Chain.POLYGON / AMOY` | `Chain` enum (137, 80002) |
| `Side.BUY / SELL` | `Side::Buy / Sell` |
| `OrderType.GTC/GTD/FOK/FAK` | `OrderType::Gtc/Gtd/Fok/Fak` |
| `SignatureTypeV2` | `SignatureType` enum (Eoa=0, Proxy=1, GnosisSafe=2, Poly1271=3) |
| `UserOrderV2` | Builder input to `OrderBuilder<Limit>` |
| `UserMarketOrderV2` | Builder input to `OrderBuilder<Market>` |
| `SignedOrderV2` | `SignedOrder` |
| `OrderV2` (EIP-712) | `Order` (alloy sol! struct) |
| `L1PolyHeader` / `L2PolyHeader` | `HeaderMap` (reqwest) |
| `OrderBookSummary` | `OrderBookSummaryResponse` |
| `Trade` | `TradeResponse` |
| `OpenOrder` | `OpenOrderResponse` |
| `number` (price/size) | `Decimal` (rust_decimal) |
| `string` (amounts in wei) | `U256` or `String` |
| `Promise<T>` | `async fn -> Result<T>` |
| `BuilderConfig` | `BuilderConfig` struct |

---

## 6. Risk & Open Questions

1. **Contract addresses** — V2 exchange contract addresses must be extracted from the TS `config.ts`. These are chain-specific and must be exact.
2. **EIP-712 struct hash** — The V2 order struct has different fields than V1. Must verify the `sol!` macro generates the correct type hash.
3. **API version detection** — The TS client queries `/version` and switches behavior. We should support this but default to V2-only.
4. **Builder fees** — The TS SDK has `BUILDER_FEES_BPS = 10000`. Need to understand how this integrates with order amounts.
5. **Backward compatibility** — Should we support V1 orders at all? The TS client does. Recommendation: V2-only initially, add V1 as a feature flag if needed.
6. **Naming** — Crate name should be chosen to not conflict with a future official Polymarket release. Consider `polymarket-clob-v2` or keep the repo name `rs-clob-client-v2`.

---

## 7. Progress Log

| Date | Phase | Status | Notes |
|---|---|---|---|
| 2026-04-19 | — | Plan drafted | Explored both reference codebases |
| 2026-04-19 | Phase 1 | Completed | Foundation crate, error model, config, serde helpers, enums, and baseline tests landed. `cargo check` passes. |
| 2026-04-19 | Phase 2 | Completed | Added auth state types, L1/L2 header generation, builder auth scaffolding, and V2 EIP-712 order struct/signing helpers. `cargo check` passes. |
| 2026-04-19 | Phase 3 | Completed | Added request/response/core model files and the type-state `Client<S>` shell with auth promotion, request execution, retry support, and cache plumbing. `cargo check` passes. |
| 2026-04-19 | Phase 4 | Completed | Implemented public read-only endpoints for health, version, markets, books, pricing, tick size / neg-risk caches, history, order book hashing, and live activity. `cargo check` passes. |
| 2026-04-19 | Phase 5 | Completed | Added authenticated key/order/trade/notification/balance/heartbeat endpoints with shared L2 header signing and pagination helpers. `cargo check` passes. |
| 2026-04-19 | Phase 6 | Completed | Implemented V2-only limit/market order builders, EIP-712 signing, order submission payloads, and cancel flows. `cargo check` passes. |
| 2026-04-19 | Phase 7 | Completed | Added builder API key/trade endpoints plus rewards and scoring APIs on the authenticated client. `cargo check` passes. |
| 2026-04-19 | Phase 8 | Completed | Added feature-gated generic WebSocket transport plus CLOB orderbook/user subscription scaffolding. `cargo check --features ws` passes. |
| 2026-04-19 | Phase 9 | Completed | Added README/examples/test coverage, fixed auth/order-builder polish issues, and verified `cargo test`, `cargo clippy -- -D warnings`, `cargo clippy --features ws -- -D warnings`, and `cargo doc --no-deps`. |
| 2026-04-19 | — | **CLOSED** | Full implementation landed. 4-way audit (architecture, code quality, security, build verification) completed. 2 P0 correctness bugs, 6 P1 issues, 7 P2 issues, and 5 P3 issues identified. Follow-up work tracked in `0002-polish-and-qa.md`. |
