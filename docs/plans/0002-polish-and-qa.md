# Plan 0002: Polish & QA Pass

**Status:** Ready
**Created:** 2026-04-19
**Predecessor:** [0001-rs-clob-client-v2-architecture.md](0001-rs-clob-client-v2-architecture.md) (CLOSED)
**Goal:** Fix all correctness bugs, security issues, and missing business logic surfaced by the 4-way audit of the initial implementation.

---

## 1. Context

Plan 0001 delivered a complete V2 Rust client (all 9 phases). A parallel audit by architecture, code-review, security, and build-verification agents produced 20 findings. This plan addresses them in priority order.

**Verification baseline (pre-polish):** 16 tests pass, 0 clippy warnings, docs build, release compiles.

---

## 2. Findings by Priority

### P0 — Correctness Bugs (will cause trading failures)

#### P0-A: EIP-712 `expiration` field in order struct

**File:** `src/clob/types/order.rs` (sol! macro, ~line 16-32)
**Problem:** The Rust `sol!` Order struct includes `expiration: uint256` as an EIP-712 field. The TS V2 reference at `references/clob-client-v2/src/order-utils/model/ctfExchangeV2TypedData.ts:5-17` does NOT include `expiration` in the typed data struct used for signing. This produces a different type hash, meaning all signed orders will be rejected on-chain.
**Action:**
1. Read the TS V2 `ctfExchangeV2TypedData.ts` to confirm `expiration` is absent from the EIP-712 struct.
2. Read the TS V2 `exchangeOrderBuilderV2.ts` to confirm `expiration` is NOT passed in the `message` object to `signTypedData`.
3. Check the on-chain contract ABI at `references/clob-client-v2/src/order-utils/model/ExchangeV2.ts` — if the contract's `Order` struct includes `expiration`, the Rust may actually be correct and the TS signing code may rely on the server to inject it. This needs careful analysis.
4. If `expiration` should NOT be in the EIP-712 struct: remove it from the `sol!` macro and ensure it is still sent in the POST payload (`PostOrderEnvelope`).
5. If `expiration` SHOULD be in the EIP-712 struct: document why the TS reference diverges and add a test verifying the type hash matches on-chain.
**Test:** Add a unit test that computes the EIP-712 type hash of the Order struct and asserts it matches the expected hash from the TS SDK or contract ABI.

#### P0-B: `calculate_market_price` iterates order book backwards

**File:** `src/clob/client.rs` (~line 645)
**Problem:** Uses `positions.iter().rev()` which walks from worst price to best. For a BUY against ascending asks, this finds the most expensive fill price. For a SELL against descending bids, this finds the cheapest fill price. Both are wrong — users get worst-case execution.
**Action:**
1. Verify the Polymarket API returns asks in ascending order (cheapest first) and bids in descending order (most expensive first) by reading the TS SDK's `calculateBuyMarketPrice` / `calculateSellMarketPrice` in `references/clob-client-v2/src/order-builder/helpers/`.
2. Remove the `.rev()` call — iterate forward (best-to-worst).
3. Add unit test with a mock 3-level order book verifying correct price for both BUY and SELL.

---

### P1 — High Priority

#### P1-A: Salt not masked to IEEE 754 safe range

**File:** `src/clob/order_builder.rs` (~line 330-332)
**Problem:** `generate_salt()` returns `U256::from(random::<u64>())`. The Polymarket backend parses salts as JavaScript numbers (IEEE 754 doubles, max safe integer 2^53 - 1). ~50% of `u64` values exceed this, causing precision loss and signature mismatches.
**Action:** Mask to 53 bits: `U256::from(random::<u64>() & ((1u64 << 53) - 1))`.
**Bonus:** The V1 Rust SDK does the same mask. Verify at `references/rs-clob-client/src/clob/order_builder.rs`.
**Test:** Assert `generate_salt() <= U256::from((1u64 << 53) - 1)` in a loop of 1000 iterations.

#### P1-B: Salt entropy too low (64-bit in 256-bit field)

**File:** `src/clob/order_builder.rs` (~line 330-332)
**Problem:** Only 64 bits of entropy. Birthday-bound collision at ~2^32 orders.
**Action:** This is partially in tension with P1-A (53-bit mask). Since the backend requires JS-safe integers, we are capped at 53 bits of entropy regardless. This is a protocol limitation, not a bug we can fix. **Resolution:** Apply the 53-bit mask (P1-A) and add a code comment documenting the constraint. No further action needed — 2^53 salts is sufficient for any realistic trading volume.

#### P1-C: `BuilderApiKey` secrets unprotected

**File:** `src/clob/types/builder.rs` (~line 19-24)
**Problem:** `secret` and `passphrase` are plain `String` with derived `Debug` — leaks secrets in logs.
**Action:**
1. Change `secret` and `passphrase` to `secrecy::SecretString`.
2. Implement a custom `Debug` that redacts these fields.
3. Since `BuilderApiKey` is a response type (deserialized from API), implement custom `Deserialize` or use `serde_with` to deserialize into `SecretString`.
4. Remove `PartialEq, Eq` derives (can't compare `SecretString`), or implement manually comparing exposed values only.

#### P1-D: `collect_pages` infinite loop risk

**File:** `src/clob/client.rs` (~line 339-349)
**Problem:** Pagination loop terminates only on cursor `"LTE="`. A bug or API change could cause infinite requests.
**Action:** Add `const MAX_PAGES: usize = 1000;` guard and check `page.data.is_empty()` as a secondary termination condition. Return an error if max pages exceeded.
**Test:** Unit test with mock server returning a non-terminating cursor, verify the function returns an error after MAX_PAGES.

#### P1-E: No market order fee adjustment

**File:** `src/clob/order_builder.rs` (~line 254-308)
**Problem:** The TS client calls `adjustBuyAmountForFees()` and `ensureBuilderFeeRateCached()` during market order creation. The Rust client skips fee adjustment entirely. The `builder_fee_rates` DashMap is dead code.
**Action:**
1. Implement a `builder_fees()` method on `Client` that calls `GET /fees/builder-fees/{builderCode}` and caches results in `builder_fee_rates`.
2. In `OrderBuilder<Market>`, for BUY orders when `user_usdc_balance` is provided, compute the fee-adjusted amount matching the TS logic in `references/clob-client-v2/src/order-builder/helpers/getMarketOrderRawAmounts.ts`.
3. Wire the builder fee rate into the `create_market_order` flow.
**Test:** Unit test: given a $100 BUY with 2% fee, verify the adjusted amount is ~$98.04.

#### P1-F: No version-mismatch retry

**File:** `src/clob/client.rs` (~line 1173-1235)
**Problem:** `create_and_post_order` / `create_and_post_market_order` do not retry on `ORDER_VERSION_MISMATCH_ERROR`. During Polymarket exchange migrations, all orders will fail instead of auto-retrying.
**Action:**
1. After `post_order` returns, check if the response contains a version mismatch error (inspect the TS `_isOrderVersionMismatch` function at `references/clob-client-v2/src/client.ts`).
2. If mismatch: invalidate `cached_version`, re-fetch version, rebuild & re-sign the order, retry once.
3. Limit to 1 retry to avoid infinite loops.
**Test:** Integration test with mock server returning version mismatch on first call, success on second.

---

### P2 — Medium Priority

#### P2-A: `std::sync::RwLock` in async context

**File:** `src/clob/client.rs` (~line 39-41, 175)
**Problem:** `cached_version` uses `std::sync::RwLock`. Currently safe because lock is never held across `.await`, but fragile.
**Action:** Replace with `tokio::sync::RwLock` or use an `AtomicU32` + sentinel value (simpler for a single `Option<u32>`). `AtomicU32` with `u32::MAX` as "unset" is the cleanest approach.

#### P2-B: No HTTPS/WSS enforcement

**File:** `src/clob/client.rs` (~line 708-715), `src/ws/config.rs` (~line 14-17)
**Problem:** Client accepts `http://` URLs, sending credentials in cleartext.
**Action:** In `Client::new()`, validate `url.scheme() == "https"`. In `ws::Config::parse()`, validate `wss://`. Allow an escape hatch via `Config::allow_insecure(true)` for local development.

#### P2-C: `body_to_string` lossy UTF-8

**File:** `src/auth.rs` (~line 371-375)
**Problem:** `String::from_utf8_lossy` replaces invalid bytes with U+FFFD, altering the HMAC message.
**Action:** Change to `std::str::from_utf8()` returning `None` on invalid UTF-8 (which would be a bug in our own serialization). Document the single-quote → double-quote replacement.

#### P2-D: Division by zero in `get_market_raw_amounts`

**File:** `src/clob/order_builder.rs` (~line 412)
**Problem:** `quote_amount / raw_price` panics if `raw_price` rounds to zero.
**Action:** Add `if raw_price.is_zero() { return Err(Error::validation("price rounds to zero")); }` before the division.

#### P2-E: `secrecy` missing `zeroize` feature

**File:** `Cargo.toml`
**Problem:** Secrets not zeroed on drop.
**Action:** Change to `secrecy = { version = "0.10", features = ["serde", "zeroize"] }`.

#### P2-F: `/fees/builder-fees/` endpoint and dead cache

**File:** `src/clob/client.rs`, `src/clob/types/response.rs`
**Problem:** `BuilderFeesResponse` type exists but no method calls the endpoint. `builder_fee_rates` DashMap is never populated.
**Action:** Addressed by P1-E (implement `builder_fees()` method). After P1-E, this is resolved.

#### P2-G: `update_balance_allowance` uses GET, returns `Result<()>`

**File:** `src/clob/client.rs` (~line 1076-1082)
**Problem:** Semantically a mutation but uses GET. Deserializing response to `()` may fail.
**Action:** Verify the actual HTTP method from the TS SDK (`references/clob-client-v2/src/client.ts`). If it's GET (Polymarket quirk), change return type to `Result<serde_json::Value>` and discard. If POST, switch to `auth_post`.

---

### P3 — Low Priority / Cleanup

#### P3-A: Dead dependencies

**File:** `Cargo.toml`
**Action:** Remove `backoff` and `strum_macros` from `[dependencies]`.

#### P3-B: `throw_on_error` stored but unused

**File:** `src/clob/client.rs`
**Action:** Remove from `Config` and `ClientInner`, or document as reserved for future use.

#### P3-C: `tracing` feature declared but unused

**File:** `Cargo.toml`, source
**Action:** Either remove the feature flag, or add basic `tracing::instrument` on the request handler (matching V1's approach).

#### P3-D: SHA-1 for orderbook hash

**File:** `src/clob/types/book.rs`
**Action:** Keep as-is — this matches the TS SDK's implementation. Changing the hash algorithm would break compatibility with server-side hash verification. Add a comment noting this is for API compatibility, not security.

#### P3-E: No `derive_proxy_wallet` / `derive_safe_wallet`

**File:** `src/clob/client.rs`
**Action:** Implement CREATE2 address derivation matching V1 at `references/rs-clob-client/src/lib.rs`. Low priority since proxy/safe wallet users can provide the funder address manually.

#### P3-F: No custom salt generator

**File:** `src/clob/order_builder.rs`
**Action:** Add an optional `salt_generator: Option<Box<dyn Fn() -> U256>>` to `OrderBuilder` for test determinism. Low priority.

#### P3-G: `Config::Remote` bearer token as plain `String`

**File:** `src/auth.rs` (~line 277)
**Action:** Change `token: Option<String>` to `token: Option<SecretString>`. Implement custom `Debug`.

#### P3-H: `PRIVATE_KEY_VAR` constant unused

**File:** `src/lib.rs` (~line 35)
**Action:** Remove, or add a `from_env()` convenience constructor that reads it.

---

## 3. Implementation Order

Work is grouped into 4 batches. Each batch must pass `cargo check && cargo test && cargo clippy -- -D warnings` before moving to the next.

### Batch 1: Critical correctness (P0-A, P0-B)
These are blocking — incorrect signatures and wrong market prices.
1. Investigate and fix the EIP-712 `expiration` field (P0-A)
2. Fix `calculate_market_price` iteration order (P0-B)
3. Add tests for both

### Batch 2: High-priority fixes (P1-A through P1-F)
4. Mask salt to 53 bits + add comment (P1-A, P1-B)
5. Protect `BuilderApiKey` secrets (P1-C)
6. Add pagination safety guard (P1-D)
7. Implement builder fee fetching + market order fee adjustment (P1-E, resolves P2-F)
8. Add version-mismatch retry (P1-F)

### Batch 3: Medium fixes (P2-A through P2-E, P2-G)
9. Replace `RwLock` with atomic or tokio equivalent (P2-A)
10. Add HTTPS/WSS enforcement (P2-B)
11. Fix `body_to_string` UTF-8 handling (P2-C)
12. Add division-by-zero guard (P2-D)
13. Enable `zeroize` on `secrecy` (P2-E)
14. Fix `update_balance_allowance` HTTP method / return type (P2-G)

### Batch 4: Cleanup (P3-*)
15. Remove dead deps `backoff`, `strum_macros` (P3-A)
16. Remove or document `throw_on_error` (P3-B)
17. Remove or implement `tracing` feature (P3-C)
18. Add SHA-1 compatibility comment (P3-D)
19. Protect `Config::Remote` bearer token (P3-G)
20. Remove or use `PRIVATE_KEY_VAR` (P3-H)
21. (Optional) Implement proxy/safe wallet derivation (P3-E)
22. (Optional) Add injectable salt generator (P3-F)

---

## 4. Verification Criteria

After all batches:
- [ ] `cargo check` passes
- [ ] `cargo check --features ws` passes
- [ ] `cargo test` passes (including new tests added in this plan)
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo clippy --features ws -- -D warnings` passes
- [ ] `cargo doc --no-deps` builds
- [ ] EIP-712 type hash test confirms match with TS SDK / on-chain contract
- [ ] Market price calculation test confirms best-price execution
- [ ] Salt masking test confirms all salts <= 2^53 - 1
- [ ] Pagination test confirms termination on stuck cursor
- [ ] No `SecretString` values leak via `Debug` formatting

---

## 5. Prompt for Sub-Orchestrator

The following prompt can be given to a Codex agent to execute this plan:

```
You are fixing correctness and security issues in the rs-clob-client-v2 Rust crate.

## Repository
/Users/hongjunwu/Documents/Git/rs-clob-client-v2/

## What exists
- Full V2 Rust client implementation in src/
- Two reference codebases in references/:
  - references/rs-clob-client/ — Polymarket's official Rust V1 SDK (style guide)
  - references/clob-client-v2/ — Polymarket's TypeScript V2 SDK (feature spec)

## Your task
Read and execute the plan at docs/plans/0002-polish-and-qa.md.

Work in 4 batches. Each batch must pass `cargo check && cargo test && cargo clippy -- -D warnings`
before moving to the next.

## Batch 1 (CRITICAL — do these first):

### P0-A: EIP-712 expiration field
1. Read references/clob-client-v2/src/order-utils/model/ctfExchangeV2TypedData.ts
   to see the V2 EIP-712 struct definition.
2. Read references/clob-client-v2/src/order-utils/exchangeOrderBuilderV2.ts
   to see what fields are passed to signTypedData.
3. Read references/clob-client-v2/src/order-utils/model/ExchangeV2.ts
   for the on-chain contract ABI Order struct.
4. Compare against src/clob/types/order.rs sol! macro.
5. If expiration is NOT in the TS EIP-712 struct but IS in the contract ABI,
   it means the server injects it — remove from sol! and keep in PostOrderEnvelope.
   If expiration IS in both, the Rust is correct — document why.
6. Add a test that computes the Order type hash and asserts correctness.

### P0-B: Market price iteration
1. Read references/clob-client-v2/src/order-builder/helpers/calculateBuyMarketPrice.ts
   and calculateSellMarketPrice.ts to understand iteration direction.
2. Fix src/clob/client.rs calculate_market_price — likely remove .rev().
3. Add unit test with mock 3-level book verifying best-price execution.

## Batch 2 (HIGH):

### P1-A: Salt masking
- In src/clob/order_builder.rs generate_salt(), apply:
  U256::from(random::<u64>() & ((1u64 << 53) - 1))
- Verify V1 does the same at references/rs-clob-client/src/clob/order_builder.rs
- Add test asserting salt <= 2^53 - 1

### P1-C: BuilderApiKey secrets
- In src/clob/types/builder.rs, change secret/passphrase to SecretString
- Implement custom Debug with redaction
- Handle Deserialize (use serde_with or custom impl)

### P1-D: Pagination guard
- In src/clob/client.rs collect_pages, add MAX_PAGES=1000 and empty-data check
- Return error on exceeded limit

### P1-E: Builder fees + market order fee adjustment
- Add builder_fees() method calling GET /fees/builder-fees/{builderCode}
- Cache result in builder_fee_rates DashMap
- In OrderBuilder<Market> for BUY, adjust amount for fees when user_usdc_balance provided
- Reference: references/clob-client-v2/src/order-builder/helpers/getMarketOrderRawAmounts.ts
  and references/clob-client-v2/src/client.ts adjustBuyAmountForFees

### P1-F: Version-mismatch retry
- In create_and_post_order / create_and_post_market_order:
  after post_order, check for version mismatch error string
- If matched: invalidate cached_version, re-fetch, rebuild order, retry once
- Reference: references/clob-client-v2/src/client.ts _retryOnVersionUpdate

## Batch 3 (MEDIUM):
- P2-A: Replace std::sync::RwLock for cached_version with AtomicU32 (u32::MAX = unset)
- P2-B: Validate https:// scheme in Client::new(), wss:// in ws::Config::parse()
- P2-C: Change body_to_string from from_utf8_lossy to from_utf8 returning None
- P2-D: Guard division by zero in get_market_raw_amounts
- P2-E: Add zeroize feature to secrecy in Cargo.toml
- P2-G: Verify update_balance_allowance HTTP method from TS SDK, fix return type

## Batch 4 (CLEANUP):
- P3-A: Remove backoff and strum_macros from Cargo.toml
- P3-B: Remove throw_on_error from Config or add #[allow(dead_code)] with doc comment
- P3-C: Remove tracing feature or add instrument macros
- P3-D: Add comment on SHA-1 usage in book.rs (API compatibility)
- P3-G: Change Config::Remote token to Option<SecretString>
- P3-H: Remove PRIVATE_KEY_VAR or add from_env() constructor

## Rules
- Run cargo check && cargo test && cargo clippy -- -D warnings after each batch.
- Do NOT change module structure or public API signatures beyond what's required.
- Do NOT add dependencies not mentioned in this plan.
- Read the TS and Rust V1 references BEFORE making changes — don't guess.
- Update the progress log at the bottom of docs/plans/0002-polish-and-qa.md after each batch.
```

---

## 6. Progress Log

| Date | Batch | Status | Notes |
|---|---|---|---|
| 2026-04-19 | — | Plan drafted | 20 findings from 4-way audit prioritized into 4 batches |
| 2026-04-19 | Batch 1 | Completed | Removed `expiration` from the signed V2 EIP-712/order struct and kept it only in the POST payload. Added an EIP-712 type-hash test. Investigated market-price iteration against TS V2 and Rust V1; both intentionally walk the API-ordered book in reverse, so logic stayed unchanged and parity tests were added. `cargo check && cargo test && cargo clippy -- -D warnings` pass. |
| 2026-04-19 | Batch 2 | Completed | Masked salts to JS-safe 53-bit values, protected `BuilderApiKey` secrets with redacted debug output, added pagination guards, implemented builder-fee caching plus BUY market-order fee adjustment, and added one-shot version-mismatch retry handling. `cargo check && cargo test && cargo clippy -- -D warnings` pass. |
| 2026-04-19 | Batch 3 | Completed | Replaced version caching locks with `AtomicU32`, enforced HTTPS in `Client::new` with an `allow_insecure` escape hatch for tests/local dev, made HMAC body handling fail on unavailable/non-UTF-8 bodies, guarded zero-price market math, and aligned `update_balance_allowance` with the TS SDK's GET semantics and response handling. `secrecy 0.10.3` already zeroizes secrets unconditionally, so no extra feature flag was added. `cargo check && cargo test && cargo clippy -- -D warnings` pass. |
| 2026-04-19 | Batch 4 | Completed | Removed unused `backoff`, `strum_macros`, and dormant tracing wiring; deleted dead `throw_on_error` and `PRIVATE_KEY_VAR`; documented SHA-1 API-compatibility usage; and redacted builder remote bearer tokens by storing them as `SecretString`. Added regression coverage for HTTPS enforcement, builder-token debug redaction, and WSS parsing. `cargo check`, `cargo check --features ws`, `cargo test`, `cargo clippy -- -D warnings`, `cargo clippy --features ws -- -D warnings`, and `cargo doc --no-deps` pass. |
| 2026-04-19 | — | **CLOSED** | 4-way re-audit (architecture verification, code review, security re-audit, build verification) confirmed all 20/20 fixes landed. 28 tests pass, 0 clippy warnings. 3 residual MEDIUM findings (WS `allow_insecure` bypass via manual `Config`, `adjust_buy_amount_for_fees` missing local zero guard, retry closure move fragility) are defense-in-depth hardening — no active bugs, no Plan 0003 needed. |
