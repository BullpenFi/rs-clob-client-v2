# Plan 0004: Parity Completion — getSigner + Integration Tests

**Status:** Ready
**Created:** 2026-04-20
**Predecessor:** [0003-audit-fixes.md](0003-audit-fixes.md) (Batches 1-4 CLOSED)
**Goal:** Achieve 100% API parity with the TypeScript V2 SDK by implementing the remaining `getSigner` callback and adding integration tests covering every gap identified between the TS test suite (17 test files) and our Rust test suite (4 test files).

---

## 1. Context

The 20-agent verification audit confirmed all Plan 0003 fixes landed with zero regressions. The only remaining TS V2 API surface gap is the `getSigner` callback pattern. The TS V2 SDK has 17 test files; we have 4. This plan closes both gaps.

**Current state:** 39 tests pass, 0 clippy warnings (59 rules), full parity on endpoints/payloads/signing/auth/math.

---

## 2. Feature: `getSigner` Callback (M6)

### What TS V2 does

```typescript
// Constructor option
getSigner?: () => Promise<ClobSigner> | ClobSigner

// Stored on OrderBuilder, NOT on ClobClient
// Used ONLY for order signing (buildOrder, buildMarketOrder)
// L1/L2 auth headers always use the static signer
// resolveSigner() prefers getSigner over static signer
// Throws if getSigner() returns null/undefined
```

Key facts:
- `getSigner` is optional — static `signer` is the fallback
- Only used in `OrderBuilder.resolveSigner()` for order signing
- L1/L2 auth headers always use the static signer passed at construction
- They are mutually exclusive: if `getSigner` exists, it takes precedence

### Rust design

Add an optional async signer factory to `OrderBuilder` that mirrors the TS pattern:

```rust
/// A factory that produces a signer on demand, enabling dynamic signer
/// resolution for smart contract wallets or rotating keys.
pub type SignerFactory = Box<dyn Fn() -> BoxFuture<'static, Result<Box<dyn Signer + Send + Sync>>> + Send + Sync>;
```

**Implementation plan:**

1. **Define `SignerFactory` type** in `src/clob/order_builder.rs`:
   ```rust
   use futures::future::BoxFuture;

   pub type SignerFactory = Box<
       dyn Fn() -> BoxFuture<'static, Result<Box<dyn Signer + Send + Sync>>>
           + Send + Sync
   >;
   ```

2. **Add `signer_factory` field** to `OrderBuilder`:
   ```rust
   pub struct OrderBuilder<OrderKind, K: AuthKind> {
       // ... existing fields ...
       signer_factory: Option<SignerFactory>,
   }
   ```

3. **Add `get_signer()` setter** on `OrderBuilder` (all variants):
   ```rust
   pub fn get_signer<F, Fut>(mut self, factory: F) -> Self
   where
       F: Fn() -> Fut + Send + Sync + 'static,
       Fut: Future<Output = Result<Box<dyn Signer + Send + Sync>>> + Send + 'static,
   {
       self.signer_factory = Some(Box::new(move || Box::pin(factory())));
       self
   }
   ```

4. **Add `resolve_signer()` method** to `OrderBuilder`:
   ```rust
   async fn resolve_signer(&self) -> Result<&dyn Signer> {
       // If signer_factory exists, call it and validate non-null
       // Otherwise fall back to self.signer
   }
   ```
   Note: This is tricky with lifetimes since the factory returns an owned signer. The implementation may need to store the resolved signer in an `Option` or return an owned type. Study the exact approach during implementation.

5. **Wire `resolve_signer()` into `build()`** for both `Limit` and `Market` variants — replace direct `self.signer` usage with `resolve_signer()`.

6. **Do NOT change `AuthenticationBuilder`** — L1/L2 auth always uses the static signer, matching TS behavior.

7. **Add `get_signer()` to `Config`** as well, so users can set it at client construction time (matching the TS constructor option). The client passes it through to `OrderBuilder` on `limit_order()` / `market_order()`.

**Tests:**
- Test with static signer (existing behavior, should not regress)
- Test with `get_signer` factory that returns a valid signer
- Test with `get_signer` factory that returns an error — verify error propagation
- Test that `get_signer` takes precedence over static signer

---

## 3. Integration Tests

### Test mapping: TS V2 tests → Rust equivalents needed

The TS V2 SDK has 17 test files. Here's what we need to add, grouped by module:

### 3a. Order Amount Calculations (`tests/order_amounts.rs`)

**TS coverage:** `buildOrderCreationArgs.test.ts`, `getOrderRawAmounts.test.ts`, `getMarketOrderRawAmounts.test.ts`, `buildMarketOrderCreationArgs.test.ts`

Add tests for `get_limit_raw_amounts` and `get_market_raw_amounts`:

| Test | Description |
|---|---|
| `limit_buy_tick_0_1` | BUY price=0.5, size=100, tick=0.1 → verify makerAmount, takerAmount |
| `limit_buy_tick_0_01` | BUY price=0.45, size=100, tick=0.01 |
| `limit_buy_tick_0_001` | BUY price=0.456, size=100, tick=0.001 |
| `limit_buy_tick_0_0001` | BUY price=0.4567, size=100, tick=0.0001 |
| `limit_sell_tick_0_1` | SELL price=0.5, size=50, tick=0.1 |
| `limit_sell_tick_0_01` | SELL price=0.60, size=50, tick=0.01 |
| `limit_sell_tick_0_001` | SELL price=0.600, size=50, tick=0.001 |
| `limit_sell_tick_0_0001` | SELL price=0.6000, size=50, tick=0.0001 |
| `market_buy_tick_0_01` | BUY amount=100 USDC, price=0.50, tick=0.01 |
| `market_sell_tick_0_01` | SELL amount=50 shares, price=0.60, tick=0.01 |
| `rounding_cascade_triggers` | Edge case where `size * price` has too many decimals → cascading round |
| `builder_code_propagation` | Verify builder code makes it into the signed order |

For each, compute expected values from the TS SDK's test fixtures or by hand using the formulas.

### 3b. Market Price Calculation (`tests/market_price.rs`)

**TS coverage:** `calculateBuyMarketPrice.test.ts`, `calculateSellMarketPrice.test.ts`

| Test | Description |
|---|---|
| `buy_price_fok_sufficient_depth` | 3-level ask book, FOK, sufficient liquidity → returns fill price |
| `buy_price_fok_insufficient_depth` | FOK with insufficient liquidity → error |
| `buy_price_fak_insufficient_depth` | FAK with insufficient liquidity → returns best price |
| `sell_price_fok_sufficient_depth` | 3-level bid book, FOK, sufficient liquidity |
| `sell_price_fok_insufficient_depth` | FOK insufficient → error |
| `sell_price_fak_insufficient_depth` | FAK insufficient → returns best price |
| `buy_accumulates_size_times_price` | Verify BUY sums `size * price` (notional) |
| `sell_accumulates_size_only` | Verify SELL sums `size` (shares) |

### 3c. Fee Calculations (`tests/fees.rs`)

**TS coverage:** `feeCalculations.test.ts`

| Test | Description |
|---|---|
| `platform_fee_at_price_0_5` | price=0.5, rate=0.02, exponent=1 → expected fee |
| `platform_fee_at_price_0_1` | price=0.1, rate=0.02, exponent=1 |
| `platform_fee_at_price_0_99` | price=0.99, rate=0.02, exponent=1 |
| `builder_fee_conversion_bps` | 200 bps → 0.02 rate |
| `adjust_buy_amount_no_adjustment` | userUsdcBalance > totalCost → amount unchanged |
| `adjust_buy_amount_with_adjustment` | userUsdcBalance ≤ totalCost → adjusted down |
| `adjust_buy_amount_zero_builder_fee` | No builder fee → only platform fee applied |

### 3d. EIP-712 Signing (`tests/signing.rs`)

**TS coverage:** `eip712.test.ts`, `order-utils/exchangeOrderBuilderV1.test.ts` (V2 portion)

| Test | Description |
|---|---|
| `deterministic_signature` | Same order + signer → same signature every time |
| `type_hash_matches_keccak` | Verify type hash = keccak256("Order(uint256 salt,...)") (already exists inline) |
| `domain_separator_polygon` | Verify domain hash for chain 137 + exchange_v2 address |
| `domain_separator_amoy` | Verify domain hash for chain 80002 |
| `neg_risk_uses_neg_risk_exchange` | neg_risk=true → verifyingContract = negRiskExchangeV2 |
| `signature_recovers_signer` | Sign → recover → assert address matches (already exists) |

### 3e. HMAC Authentication (`tests/auth.rs` — expand existing)

**TS coverage:** `hmac.test.ts`, `headers/index.test.ts`

| Test | Description |
|---|---|
| `hmac_known_vector` | Known secret + message → expected signature (deterministic) |
| `hmac_empty_body` | GET request (no body) → correct message format |
| `hmac_with_body` | POST request with JSON body → correct message format |
| `l1_headers_contain_all_fields` | Verify POLY_ADDRESS, POLY_SIGNATURE, POLY_TIMESTAMP, POLY_NONCE all present |
| `l2_headers_contain_all_fields` | Verify POLY_ADDRESS, POLY_SIGNATURE, POLY_TIMESTAMP, POLY_API_KEY, POLY_PASSPHRASE |
| `l1_address_is_lowercase` | Verify POLY_ADDRESS is lowercase hex (already exists) |
| `secret_with_plus_slash` | Standard base64 secret with +/ characters (already exists) |

### 3f. Client Endpoint Integration (`tests/client.rs` — expand existing)

**TS coverage:** implicit in `client.ts` methods

| Test | Description |
|---|---|
| `server_time_returns_timestamp` | Mock `/time` → verify parsed response |
| `version_returns_and_caches` | Mock `/version` → verify caching with AtomicU32 |
| `midpoint_returns_decimal` | Mock `/midpoint` → verify Decimal parsing |
| `price_returns_side_and_price` | Mock `/price` → verify response shape |
| `order_book_returns_bids_asks` | Mock `/book` → verify OrderBookSummary deserialization |
| `post_order_deserializes_camel_case` | Mock `/order` POST → verify OrderResponse (already partially exists) |
| `cancel_order_sends_order_id` | Mock DELETE `/order` → verify payload has `orderID` |
| `cancel_all_sends_empty_body` | Mock DELETE `/cancel-all` → verify no body |
| `pagination_collects_all_pages` | Mock paginated `/data/orders` → verify collection |
| `pagination_stops_on_empty_page` | Mock empty page → verify early termination |
| `balance_allowance_returns_values` | Mock `/balance-allowance` → verify response |
| `create_or_derive_falls_back` | Mock failing create + successful derive → verify fallback |
| `version_mismatch_retries_once` | Already exists |

### 3g. Serde Round-Trip (`tests/serde.rs`)

| Test | Description |
|---|---|
| `side_buy_serializes_as_string` | `Side::Buy` → `"BUY"` |
| `side_sell_serializes_as_string` | `Side::Sell` → `"SELL"` |
| `signature_type_serializes_as_number` | `SignatureTypeV2::Eoa` → `0` |
| `order_type_serializes_as_string` | `OrderType::Gtc` → `"GTC"` |
| `tick_size_round_trip` | Each tick size survives serialize → deserialize |
| `market_details_shorthand_fields` | Deserialize `{"c":"...","mts":0.01,"nr":false}` |
| `open_order_all_fields` | Full OpenOrder JSON → struct → verify all fields |
| `trade_all_fields` | Full Trade JSON → struct → verify all fields |
| `order_book_summary_hash` | Serialize OrderBookSummary → SHA-1 hash → verify string format |

---

## 4. Implementation Order

### Batch 1: `getSigner` feature (M6)

1. Define `SignerFactory` type in `order_builder.rs`
2. Add `signer_factory` field to `OrderBuilder`
3. Add `get_signer()` setter method
4. Implement `resolve_signer()` with fallback logic
5. Wire into `Limit::build()` and `Market::build()`
6. Add `get_signer` to client `Config` and pass through
7. Add tests (static fallback, factory precedence, error propagation)

### Batch 2: Core math tests

8. Add `tests/order_amounts.rs` — 12 tests for limit/market amounts across tick sizes
9. Add `tests/market_price.rs` — 8 tests for FOK/FAK buy/sell price calculation
10. Add `tests/fees.rs` — 7 tests for fee formulas

### Batch 3: Signing + auth tests

11. Add `tests/signing.rs` — 6 tests for EIP-712 determinism, domain, recovery
12. Expand `tests/auth.rs` — add HMAC known vectors, empty/body message tests

### Batch 4: Client + serde tests

13. Expand `tests/client.rs` — 12 endpoint integration tests with httpmock
14. Add `tests/serde.rs` — 9 serde round-trip tests

---

## 5. Verification Criteria

After all batches:
- [ ] `cargo check` passes
- [ ] `cargo check --features ws` passes
- [ ] `cargo test` passes with **80+ tests** (currently 39)
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo clippy --features ws -- -D warnings` passes
- [ ] `cargo doc --no-deps` builds
- [ ] `getSigner` callback works with async factory
- [ ] `getSigner` takes precedence over static signer
- [ ] `getSigner` error propagates correctly
- [ ] Order amounts verified for all 4 tick sizes, BUY and SELL
- [ ] Market price verified for FOK and FAK modes
- [ ] Fee calculations verified at multiple price points
- [ ] All endpoint mocks verify correct path, method, auth level, and payload

---

## 6. Prompt for Sub-Orchestrator

```
You are adding the final feature and comprehensive tests to rs-clob-client-v2.

## Repository
/Users/hongjunwu/Documents/Git/rs-clob-client-v2/

## Layout
src/                          # Working code
references/clob-client-v2/    # TypeScript V2 SDK — READ ONLY
docs/plans/0004-parity-completion.md  # THIS PLAN — read first

## How to work
- Read docs/plans/0004-parity-completion.md fully before touching code.
- Work in 4 batches. After EACH batch, run:
  cargo check && cargo test && cargo clippy -- -D warnings
- Read the TS reference files BEFORE implementing. Do not guess.
- Update the progress log after each batch.

## Batch 1: getSigner callback (M6)

The TS V2 SDK supports a getSigner callback for dynamic signer resolution.
Read these files to understand the pattern:
- references/clob-client-v2/src/client.ts — constructor option, how it's passed to OrderBuilder
- references/clob-client-v2/src/order-builder/orderBuilder.ts — resolveSigner() method

Key constraints:
- getSigner is used ONLY for order signing (OrderBuilder.build)
- L1/L2 auth headers always use the static signer — do NOT change auth.rs
- If getSigner exists, it takes precedence over the static signer
- If getSigner returns an error, propagate it
- The factory is async (returns a Future)

Implementation in src/clob/order_builder.rs:
1. Define a SignerFactory type (boxed async closure returning a boxed Signer)
2. Add signer_factory: Option<SignerFactory> to OrderBuilder
3. Add get_signer() setter that accepts a generic closure
4. Add resolve_signer() that prefers factory over static signer
5. Wire resolve_signer() into Limit::build() and Market::build()
6. Also add to Config in client.rs so users can set it at construction

Tests:
- Static signer works (existing behavior must not regress)
- Factory signer takes precedence
- Factory error propagates
- Factory returns valid signer → order builds successfully

## Batch 2: Core math tests (tests/order_amounts.rs, tests/market_price.rs, tests/fees.rs)

Create three new test files. Each test should use concrete numerical values
and assert exact expected outputs.

For order amounts: test all 4 tick sizes (0.1, 0.01, 0.001, 0.0001) for both
BUY and SELL. Compute expected makerAmount/takerAmount by hand or from the
TS test fixtures at references/clob-client-v2/tests/order-builder/.

For market prices: use mock 3-level orderbooks and verify correct fill prices
for both FOK and FAK modes, BUY and SELL.

For fees: test the platform fee formula at multiple price points and the
builder fee adjustment.

## Batch 3: Signing + auth tests (tests/signing.rs, expand tests/auth.rs)

For signing: test EIP-712 type hash, domain separator for Polygon and Amoy,
neg_risk exchange routing, deterministic signatures, sign-then-recover.

For auth: add HMAC known-vector tests with pre-computed expected values,
empty body message format, POST body message format.

## Batch 4: Client + serde tests (expand tests/client.rs, tests/serde.rs)

For client: add httpmock-based integration tests for major endpoints
(server_time, midpoint, order_book, post_order, cancel_order, cancel_all,
pagination, balance_allowance, create_or_derive_api_key).

For serde: test enum serialization round-trips, MarketDetails shorthand,
OpenOrder/Trade full-field deserialization, OrderBookSummary hash.

## Rules
- cargo check && cargo test && cargo clippy -- -D warnings after each batch
- New test files need mod common; for shared helpers
- Use httpmock for all HTTP integration tests
- Use concrete numerical values, not random — tests must be deterministic
- Read TS test fixtures for expected values where available
- Do not modify production code except for the getSigner feature in Batch 1
```

---

## 7. Progress Log

| Date | Batch | Status | Notes |
|---|---|---|---|
| 2026-04-20 | — | Plan drafted | 1 feature (getSigner) + ~54 integration tests across 7 test files |
| 2026-04-20 | Batch 1 | Completed | Added async `get_signer` support on `OrderBuilder` and `Config`, resolved signer precedence in order creation flows, and added 4 parity tests in `tests/get_signer.rs` |
| 2026-04-20 | Batch 2 | Completed | Added 12 order amount tests, 8 market price tests, and 7 fee tests in `tests/order_amounts.rs`, `tests/market_price.rs`, and `tests/fees.rs` |
| 2026-04-20 | Batch 3 | Completed | Added 6 EIP-712 signing tests in `tests/signing.rs` and expanded `tests/auth.rs` with fixed L1/L2 signature vectors plus body/no-body header coverage |
| 2026-04-20 | Batch 4 | Completed | Expanded `tests/client.rs` with endpoint/cache/pagination coverage, added `tests/serde.rs`, and passed final `cargo check`, `cargo check --features ws`, `cargo test`, clippy, and rustdoc verification |
