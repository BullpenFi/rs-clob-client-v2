# Plan 0003: Audit Fixes — Convention & Correctness

**Status:** Ready
**Created:** 2026-04-19
**Predecessor:** [0002-polish-and-qa.md](0002-polish-and-qa.md) (CLOSED)
**Goal:** Fix all findings from the 20-agent audit (10 convention agents comparing against Rust V1 SDK, 10 correctness agents comparing against TypeScript V2 SDK).

---

## 1. Context

Plan 0002 closed with all 20 original findings fixed. A subsequent deep audit spawned 20 parallel agents comparing our implementation against both reference codebases. The audit found **2 CRITICAL** production-blocking bugs, **7 HIGH** issues, **10 MEDIUM** items, and **13 LOW/cleanup** items.

**Verification baseline:** 28 tests pass, 0 clippy warnings, docs build, release compiles.

---

## 2. Findings

### CRITICAL (will cause runtime failures)

#### C1: Wrong base64 alphabet for HMAC secret decoding

**File:** `src/auth.rs:412`
**Problem:** `URL_SAFE.decode(secret.expose_secret())` uses URL-safe base64 alphabet (`-_`). The TS SDK uses standard base64 (`Buffer.from(secret, "base64")`) which expects `+/`. If the Polymarket API returns secrets containing `+` or `/`, `URL_SAFE.decode()` returns a `DecodeError`, breaking all L2 authenticated requests.
**Evidence:** TS `references/clob-client-v2/src/signing/hmac.ts` uses `Buffer.from(secret, "base64")` (standard). The output encoding (`URL_SAFE.encode`) is correct — TS manually replaces `+`→`-`, `/`→`_` on the output, matching `URL_SAFE.encode`.
**Fix:** Change the decode call to use `STANDARD`:
```rust
use base64::engine::general_purpose::{STANDARD, URL_SAFE};
let decoded_secret = STANDARD.decode(secret.expose_secret())?;
// ... keep URL_SAFE.encode(result) for output
```
**Test:** Add a unit test with a secret containing `+` and `/` characters, verify HMAC computation succeeds and matches a known-good output.

#### C2: `OrderResponse` fields missing serde renames

**File:** `src/clob/types/response.rs:122-134`
**Problem:** The struct has fields `error_msg`, `order_id`, `transactions_hashes`, `taking_amount`, `making_amount` with no `#[serde(rename)]` attributes. The Polymarket API sends these as camelCase (`errorMsg`, `orderID`, `transactionsHashes`, `takingAmount`, `makingAmount`). Every order submission response will fail to deserialize.
**Evidence:** TS `references/clob-client-v2/src/types/clob.ts:57-65` — all fields are camelCase. Note `orderID` is unusual casing (not standard camelCase `orderId`), so `rename_all = "camelCase"` alone won't work.
**Fix:** Add individual renames:
```rust
#[serde(rename = "errorMsg")]
pub error_msg: Option<String>,
#[serde(rename = "orderID")]
pub order_id: String,
#[serde(rename = "transactionsHashes")]
pub transactions_hashes: Vec<String>,
#[serde(rename = "takingAmount")]
pub taking_amount: String,
#[serde(rename = "makingAmount")]
pub making_amount: String,
```
**Test:** Add a unit test deserializing a JSON string matching the API shape, verify all fields are populated.

---

### HIGH (incorrect behavior or significant gap)

#### H1: Address encoding — checksummed vs lowercase

**File:** `src/auth.rs:199` (L1), `src/auth.rs:231` (L2)
**Problem:** `signer.address().to_string()` produces EIP-55 checksummed mixed-case hex (e.g., `0xF39Fd6e51...`). The V1 Rust SDK uses `encode_hex_with_prefix()` which produces all-lowercase (e.g., `0xf39fd6e51...`). The `POLY_ADDRESS` header value may be rejected if the Polymarket API compares case-sensitively.
**Evidence:** V1 Rust `references/rs-clob-client/src/auth.rs:208-209` uses `encode_hex_with_prefix()`. V1 test at line 460 asserts all-lowercase.
**Fix:**
1. Add `use alloy::hex::ToHexExt as _;` to both `l1` and `l2` modules.
2. Change `signer.address().to_string()` to `signer.address().encode_hex_with_prefix()` in L1 `create_headers`.
3. Change `state.address.to_string()` to `state.address.encode_hex_with_prefix()` in L2 `create_headers`.
**Test:** Update existing L1/L2 header tests to assert the address value is all-lowercase.

#### H2: Missing default HTTP headers

**File:** `src/clob/client.rs:748`
**Problem:** `ReqwestClient::new()` sets no default headers. The TS SDK sets `User-Agent: @polymarket/clob-client`, `Accept: */*`, `Connection: keep-alive`, `Content-Type: application/json`.
**Evidence:** TS `references/clob-client-v2/src/http-helpers/index.ts:12-28` (node-only headers).
**Fix:** Replace `ReqwestClient::new()` with:
```rust
let mut headers = HeaderMap::new();
headers.insert("User-Agent", HeaderValue::from_static("polymarket-clob-client-v2"));
headers.insert("Accept", HeaderValue::from_static("*/*"));
headers.insert("Connection", HeaderValue::from_static("keep-alive"));
headers.insert("Content-Type", HeaderValue::from_static("application/json"));
ReqwestClient::builder().default_headers(headers).build()?
```
**Test:** Integration test with httpmock verifying the `User-Agent` header is sent.

#### H3: Retry scope mismatch — retries GET/DELETE but TS only retries POST

**File:** `src/clob/client.rs:216,863`
**Problem:** When `retry_on_error` is set, the Rust client retries all HTTP methods. The TS SDK only retries POST requests.
**Evidence:** TS `references/clob-client-v2/src/http-helpers/index.ts:51-84` — only the `post` helper has retry logic. `get` and `del` never retry.
**Fix:** Only pass `retry_on_error: true` when the request method is POST. In `request_json` and `auth_request`, check the method:
```rust
let should_retry = self.config.retry_on_error && method == Method::POST;
```
**Test:** Integration test verifying a 500 on GET is NOT retried, but a 500 on POST IS retried.

#### H4: Missing funder/signature_type cross-validation

**File:** `src/clob/client.rs:98-143` (in `AuthenticationBuilder::authenticate`)
**Problem:** V1 rejects `EOA + funder` (nonsensical) and `Proxy/GnosisSafe + zero funder` (would sign for wrong address). V2 allows any combination silently.
**Evidence:** V1 `references/rs-clob-client/src/clob/client.rs:170-186` — explicit validation with descriptive error messages.
**Fix:** Add validation before constructing the authenticated client:
```rust
if signature_type == SignatureTypeV2::Eoa && funder.is_some() {
    return Err(Error::validation("funder address is not supported with EOA signature type"));
}
if matches!(signature_type, SignatureTypeV2::Proxy | SignatureTypeV2::GnosisSafe)
    && funder.map_or(true, |f| f.is_zero())
{
    return Err(Error::validation("non-zero funder address is required for Proxy/GnosisSafe signature types"));
}
```

#### H5: Lint configuration gap

**File:** `Cargo.toml:56-60`
**Problem:** V2 has 4 lint rules. V1 has 50+ including safety-critical lints like `unwrap_used`, `dbg_macro`, `todo`, `print_stdout`, `string_slice`.
**Evidence:** V1 `references/rs-clob-client/Cargo.toml:212-274`.
**Fix:** Copy V1's lint block into V2's `Cargo.toml`. Then run `cargo clippy` and fix any new warnings. Key lints to add at minimum:
```toml
[lints.clippy]
pedantic = { level = "warn", priority = -1 }
cargo = { level = "warn", priority = -1 }
unwrap_used = "warn"
dbg_macro = "warn"
todo = "warn"
print_stdout = "warn"
print_stderr = "warn"
string_slice = "warn"
get_unwrap = "warn"
undocumented_unsafe_blocks = "warn"
```
**Note:** After adding these, existing code may produce warnings. Fix them as part of this batch.

#### H6: `OrderBookSummary.hash()` produces wrong hash

**File:** `src/clob/types/book.rs:96-104`
**Problem:** `hash()` serializes the struct to JSON and SHA-1 hashes it. `OrderSummary.price` and `OrderSummary.size` are `Decimal`, which with `rust_decimal`'s `serde` feature serializes as a JSON number (e.g., `0.5`). The TS SDK uses strings (e.g., `"0.5"`). The hash will never match the server's hash.
**Evidence:** TS `references/clob-client-v2/src/types/clob.ts:71-77` — `price: string`, `size: string`.
**Fix:** Add `#[serde_as(as = "DisplayFromStr")]` or `#[serde(serialize_with = "...")]` on `OrderSummary.price` and `OrderSummary.size` to serialize as strings. Alternatively, add `serde-with-str` feature to `rust_decimal` in `Cargo.toml` — but this would affect ALL `Decimal` fields. Prefer per-field annotation:
```rust
#[serde_as]
pub struct OrderSummary {
    #[serde_as(as = "DisplayFromStr")]
    pub price: Decimal,
    #[serde_as(as = "DisplayFromStr")]
    pub size: Decimal,
}
```
**Test:** Serialize an `OrderSummary` and verify the JSON contains string values for price/size, not numbers.

#### H7: Missing gzip support

**File:** `Cargo.toml:38`
**Problem:** reqwest features don't include `gzip`. The TS SDK sends `Accept-Encoding: gzip` for GET requests. Without gzip, the Rust client uses significantly more bandwidth for large market data responses.
**Evidence:** TS `references/clob-client-v2/src/http-helpers/index.ts:26-27`.
**Fix:** Add `"gzip"` to reqwest features:
```toml
reqwest = { version = "0.13.2", default-features = false, features = ["json", "query", "rustls", "gzip"] }
```

---

### MEDIUM (consider fixing)

#### M1: `body_to_string` single-quote replacement has no TS equivalent

**File:** `src/auth.rs:408`
**Problem:** `body.replace('\'', "\"")` mutates the HMAC message. The TS SDK has no such transformation. If a JSON value legitimately contains a single quote (e.g., `{"name": "O'Brien"}`), the HMAC message would differ from what the server computes.
**Fix:** Remove the `.replace('\'', "\"")` call. `serde_json` never produces single quotes, so this is a no-op for well-formed JSON, but it's a correctness risk for edge cases.

#### M2: Missing price decimal-places validation

**File:** `src/clob/order_builder.rs:180-190`
**Problem:** V1 rejects prices with more decimal places than the tick size allows (e.g., price=0.345 with tick_size=0.01). V2 silently rounds the price via `round_normal` in `get_limit_raw_amounts`, which may surprise users.
**Fix:** Add a check before the range validation:
```rust
if price.normalize().scale() > tick_size.as_decimal().normalize().scale() {
    return Err(Error::validation(format!(
        "price has too many decimal places for tick size {tick_size}"
    )));
}
```

#### M3: Missing `Amount` type (USDC vs Shares distinction)

**File:** `src/clob/order_builder.rs:62`
**Problem:** V1 has `Amount` type distinguishing `Amount::Usdc(n)` vs `Amount::Shares(n)` for market orders. V2 always treats market BUY amount as USDC and SELL amount as shares. The TS V2 SDK also lacks this distinction, so this is a V2 design choice.
**Action:** Document this as intentional. Add a doc comment on `OrderBuilder<Market>::amount()` explaining the semantics: "For BUY orders, amount is in USDC. For SELL orders, amount is in shares."

#### M4: `post_only` changed from `Option<bool>` to `bool`

**File:** `src/clob/types/order.rs:40`
**Problem:** V1 uses `Option<bool>` and omits the field from JSON when `None`. V2 always sends `post_only: false` for market orders. The TS V2 SDK also always sends the field.
**Action:** Verify the API accepts `post_only: false` for FOK/FAK orders. If so, no change needed — document as intentional.

#### M5: `createOrDeriveApiKey` fallback scope

**File:** `src/clob/client.rs:276`
**Problem:** Rust falls back to `derive_api_key` only on `ErrorKind::Status` errors. TS falls back on any error (including network errors) because its HTTP layer returns `{ error }` objects rather than throwing.
**Fix:** Consider widening the fallback match:
```rust
Err(_) => self.derive_api_key(signer, nonce).await,
```
Or keep current behavior and document: "Unlike the TS SDK, network errors during create_api_key are not retried via derive_api_key."

#### M6: No `getSigner` callback for dynamic signer resolution

**Problem:** TS V2 supports `getSigner?: () => Promise<ClobSigner>` for smart contract wallets with rotating signers. Rust requires a concrete `&S: Signer` reference.
**Action:** Document as a known limitation. Consider adding a trait-based signer factory in a future release if the use case is needed.

#### M7: Missing tracing feature

**Problem:** V1 has a `tracing` feature with `serde_ignored` + `serde_path_to_error` for unknown-field detection and structured request logging. V2 has no observability.
**Action:** Add as optional feature. At minimum, add `#[cfg_attr(feature = "tracing", tracing::instrument(skip_all))]` on the request handler. Add to Cargo.toml:
```toml
tracing = ["dep:tracing", "dep:serde_ignored", "dep:serde_path_to_error"]
```
And add the three optional deps. Wire `deserialize_with_warnings` to use them when the feature is active.

#### M8: Missing `Clone` on most request types

**File:** `src/clob/types/request.rs`
**Problem:** V1 derives `Clone` on all request types. V2 omits it on most. Users can't reuse request structs.
**Fix:** Add `Clone` to the derive list on all request types in `request.rs`.

#### M9: Missing `Serialize` on many response types

**File:** `src/clob/types/response.rs`
**Problem:** V1 derives both `Serialize` and `Deserialize` on response types. V2 often only derives `Deserialize`. Users can't serialize responses for caching/logging.
**Fix:** Add `Serialize` to the derive list on all response types.

#### M10: Missing retry error codes (DNS failures)

**File:** `src/lib.rs:106`
**Problem:** TS retries on `ECONNABORTED`, `ENETUNREACH`, `EAI_AGAIN`, `ETIMEDOUT`. Rust only checks `is_connect() || is_timeout()`, which may miss DNS resolution failures.
**Fix:** Also check `error.is_request()` for broader coverage:
```rust
if error.is_connect() || error.is_timeout() || error.is_request() {
```

---

### LOW / Cleanup

#### L1: Missing `downcast_ref` on `Error`

**File:** `src/error.rs`
**Fix:** Add:
```rust
pub fn downcast_ref<E: StdError + 'static>(&self) -> Option<&E> {
    self.source.as_deref()?.downcast_ref::<E>()
}
```

#### L2: Missing error module unit tests

**File:** `src/error.rs`
**Fix:** Add tests matching V1's `error.rs:283-313` — `Geoblock` display and `From` conversion tests.

#### L3: Missing `PRIVATE_KEY_VAR` constant

**File:** `src/lib.rs`
**Fix:** Add `pub const PRIVATE_KEY_VAR: &str = "POLYMARKET_PRIVATE_KEY";`

#### L4: Missing wallet derivation functions

**File:** `src/lib.rs` or `src/config.rs`
**Action:** Implement `derive_proxy_wallet()` and `derive_safe_wallet()` using CREATE2 address derivation, matching V1 `references/rs-clob-client/src/lib.rs:163-197`. Add `WalletContractConfig` and `WALLET_CONFIG` phf_map. Alternatively, defer and document as out-of-scope for V2 initial release.

#### L5: Missing `dec!` macro re-export

**File:** `src/types.rs`, `Cargo.toml`
**Fix:** Add `rust_decimal_macros` to `[dependencies]` and `pub use rust_decimal_macros::dec;` to `types.rs`.

#### L6: Blanket crate-level clippy allows

**File:** `src/lib.rs:1-7`
**Fix:** Remove the `#![allow(...)]` block. Address individual warnings at their source with per-item `#[allow(...)]` if truly needed. Most should be fixable by adjusting the code.

#### L7: Missing doc comments throughout

**Files:** `src/clob/mod.rs`, `src/types.rs`, `src/auth.rs`, `src/error.rs`
**Fix:** Add module-level `//!` doc comments to `clob/mod.rs` (V1 has 145 lines of module docs with endpoint tables and code examples). Add `///` doc comments on `Kind` variants, `Credentials`, `types.rs` re-exports. Reference V1 for content.

#### L8: Missing `tests/common/mod.rs`

**File:** `tests/`
**Fix:** Create `tests/common/mod.rs` with shared constants (`PRIVATE_KEY`, `SECRET`, `PASSPHRASE`), `signer()` helper, and `create_authenticated()` async helper. Update `tests/auth.rs`, `tests/order.rs`, `tests/client.rs` to use `mod common;`.

#### L9: Test coverage gap (18 vs 351)

**Action:** This is a long-term effort, not a single batch. Prioritize:
1. CLOB endpoint integration tests with httpmock (markets, orderbook, pricing, orders, trades, cancel, notifications, balance, rewards)
2. Order builder validation tests (all 4 tick sizes, BUY/SELL, edge cases)
3. Auth lifecycle integration tests (create/derive/delete API keys with mocked HTTP)

#### L10: No benchmarks

**Action:** Add `benches/` with criterion benchmarks for order building, order signing, and response deserialization. Add `criterion` to `[dev-dependencies]`. Low priority.

#### L11: WebSocket layer is a stub

**Action:** Not addressed in this plan. Track separately — requires significant design work (reconnection, broadcast channels, typed messages, subscription management).

#### L12: `taker` field missing from POST payload

**File:** `src/clob/client.rs:1575-1593`
**Action:** The TS V2 `OrderV2` interface does NOT define a `taker` field. The `orderToJsonV2` sets `taker: order.taker` which is `undefined` at runtime and omitted from JSON. Our omission is correct. No fix needed — add a comment documenting this.

#### L13: `tokio` is non-optional

**File:** `Cargo.toml:48`
**Action:** Consider making `tokio` optional, gated behind `ws` feature. The `time` feature is used for retry sleeps in `lib.rs`. If retry is always available, `tokio` must be non-optional. Document as intentional.

---

## 3. Implementation Order

Work is grouped into 5 batches. Each batch must pass `cargo check && cargo test && cargo clippy -- -D warnings` before moving to the next.

### Batch 1: CRITICAL production blockers (C1, C2)

1. **C1:** Fix base64 decode alphabet — change `URL_SAFE` to `STANDARD` for secret decoding in `auth.rs`. Add test with `+`/`/` secret.
2. **C2:** Add `#[serde(rename)]` to all 5 camelCase fields in `OrderResponse`. Add deserialization test.

### Batch 2: HIGH correctness fixes (H1, H2, H3, H4, H6, H7)

3. **H1:** Switch address encoding to `encode_hex_with_prefix()` in L1 and L2 `create_headers`. Update tests.
4. **H2:** Configure `ReqwestClient::builder()` with default headers (User-Agent, Accept, Connection, Content-Type).
5. **H3:** Only retry POST requests — add method check in retry logic.
6. **H4:** Add funder/signature_type cross-validation in `AuthenticationBuilder::authenticate`.
7. **H6:** Fix `OrderSummary` serialization — add `#[serde_as(as = "DisplayFromStr")]` on `price` and `size`.
8. **H7:** Add `"gzip"` to reqwest features in `Cargo.toml`.

### Batch 3: HIGH code quality + MEDIUM fixes (H5, M1-M5, M8-M10)

9. **H5:** Adopt V1's lint configuration. Fix resulting clippy warnings.
10. **M1:** Remove `body_to_string` single-quote replacement.
11. **M2:** Add price decimal-places validation in order builder.
12. **M3:** Document `Amount` semantics on `OrderBuilder<Market>::amount()`.
13. **M4:** Verify `post_only: false` is accepted by API. Document.
14. **M5:** Decide on `createOrDeriveApiKey` fallback scope — widen or document.
15. **M8:** Add `Clone` to all request types.
16. **M9:** Add `Serialize` to all response types.
17. **M10:** Add `error.is_request()` to retry conditions.

### Batch 4: LOW fixes (L1-L8, L12)

18. **L1:** Add `downcast_ref` to `Error`.
19. **L2:** Add error module unit tests.
20. **L3:** Add `PRIVATE_KEY_VAR` constant.
21. **L5:** Add `rust_decimal_macros` dep and `dec!` re-export.
22. **L6:** Remove blanket clippy allows, fix per-item.
23. **L7:** Add doc comments to `clob/mod.rs`, `types.rs`, `auth.rs`, `error.rs` Kind variants.
24. **L8:** Create `tests/common/mod.rs` with shared helpers.
25. **L12:** Add comment explaining `taker` omission is intentional.

### Batch 5: Deferred / Long-term (L4, L9, L10, L11, L13, M6, M7)

These are tracked but not blocking. Implement as capacity allows:

26. **M7:** Add `tracing` feature with `serde_ignored` + `serde_path_to_error` integration.
27. **L4:** Implement wallet derivation functions (or document as out-of-scope).
28. **L9:** Expand integration test coverage toward V1 parity.
29. **L10:** Add criterion benchmarks.
30. **L11:** WebSocket layer redesign (separate plan).
31. **L13:** Evaluate making `tokio` optional.
32. **M6:** Evaluate `getSigner` callback pattern.

---

## 4. Verification Criteria

After Batches 1-4:
- [ ] `cargo check` passes
- [ ] `cargo check --features ws` passes
- [ ] `cargo test` passes (including all new tests)
- [ ] `cargo clippy -- -D warnings` passes (with V1-level lint config)
- [ ] `cargo clippy --features ws -- -D warnings` passes
- [ ] `cargo doc --no-deps` builds
- [ ] HMAC test with `+`/`/` in secret passes
- [ ] `OrderResponse` deserialization from camelCase JSON passes
- [ ] L1/L2 auth header address values are all-lowercase hex
- [ ] `OrderBookSummary.hash()` test confirms string-based Decimal serialization
- [ ] Retry test confirms GET is NOT retried, POST IS retried
- [ ] Default HTTP headers test confirms `User-Agent` is sent

---

## 5. Prompt for Sub-Orchestrator

```
You are fixing correctness bugs and convention gaps in the rs-clob-client-v2 Rust crate.

## Repository
/Users/hongjunwu/Documents/Git/rs-clob-client-v2/

## Layout
src/                          # Your working code
references/rs-clob-client/    # Polymarket's official Rust V1 SDK — READ ONLY
references/clob-client-v2/    # Polymarket's TypeScript V2 SDK — READ ONLY
docs/plans/0003-audit-fixes.md   # THE PLAN — read this FIRST

## How to work
- Read docs/plans/0003-audit-fixes.md fully before touching code.
- Work in 4 batches (1-4). Batch 5 is deferred.
- After EACH batch, run:
  cargo check && cargo test && cargo clippy -- -D warnings
- Fix any failures before moving to the next batch.
- Read the reference files BEFORE making changes. Do not guess.
- Update the progress log at the bottom of docs/plans/0003-audit-fixes.md after each batch.

## Batch 1: CRITICAL (do these first)

### C1: Fix base64 decode alphabet in HMAC
File: src/auth.rs (~line 412)
- Change URL_SAFE.decode to STANDARD.decode for the secret
- Keep URL_SAFE.encode for the output signature
- Add import: use base64::engine::general_purpose::{STANDARD, URL_SAFE};
- Reference: references/clob-client-v2/src/signing/hmac.ts — Buffer.from(secret, "base64") uses standard base64
- Add test: create a secret containing + and / chars, verify HMAC succeeds

### C2: Fix OrderResponse serde renames
File: src/clob/types/response.rs (~line 122-134)
- Add #[serde(rename = "errorMsg")] on error_msg
- Add #[serde(rename = "orderID")] on order_id
- Add #[serde(rename = "transactionsHashes")] on transactions_hashes
- Add #[serde(rename = "takingAmount")] on taking_amount
- Add #[serde(rename = "makingAmount")] on making_amount
- Reference: references/clob-client-v2/src/types/clob.ts lines 57-65
- Add test: deserialize {"success":true,"errorMsg":"","orderID":"abc","transactionsHashes":[],"status":"live","takingAmount":"100","makingAmount":"50"} and verify all fields

## Batch 2: HIGH correctness fixes

### H1: Fix address encoding to lowercase
File: src/auth.rs (~line 199 and ~line 231)
- Add `use alloy::hex::ToHexExt as _;` to both l1 and l2 modules
- Change .to_string() to .encode_hex_with_prefix() for address values
- Reference: references/rs-clob-client/src/auth.rs:208-209
- Update existing auth tests to assert lowercase addresses

### H2: Add default HTTP headers
File: src/clob/client.rs (~line 748)
- Use ReqwestClient::builder().default_headers(headers).build()?
- Set User-Agent, Accept, Connection, Content-Type
- Reference: references/clob-client-v2/src/http-helpers/index.ts:12-28

### H3: Only retry POST requests
File: src/clob/client.rs (~line 216 and ~line 863)
- Pass retry_on_error only when method == Method::POST
- Reference: references/clob-client-v2/src/http-helpers/index.ts:51-84

### H4: Add funder/signature_type validation
File: src/clob/client.rs (~line 98-143, in authenticate())
- Reject EOA + funder, Proxy/GnosisSafe + zero/missing funder
- Reference: references/rs-clob-client/src/clob/client.rs:170-186

### H6: Fix OrderSummary serialization for hash
File: src/clob/types/book.rs
- Add #[serde_as(as = "DisplayFromStr")] on OrderSummary.price and .size
- Add #[serde_as] on the struct
- Add test: serialize OrderSummary and verify price/size are strings in JSON

### H7: Add gzip support
File: Cargo.toml (~line 38)
- Add "gzip" to reqwest features list

## Batch 3: Lints + MEDIUM fixes

### H5: Adopt V1 lint config
File: Cargo.toml (~line 56-60)
- Replace the minimal lint block with V1's comprehensive lint config
- Reference: references/rs-clob-client/Cargo.toml:212-274
- Fix all resulting clippy warnings in src/

### M1: Remove single-quote replacement in body_to_string
File: src/auth.rs (~line 408)
- Remove .replace('\'', "\"")

### M2: Add price decimal-places validation
File: src/clob/order_builder.rs (~line 180-190)
- Add check: price.normalize().scale() > tick_size scale → error

### M3: Document Amount semantics
File: src/clob/order_builder.rs — add doc comment on amount() method

### M5: Decide createOrDeriveApiKey fallback scope
File: src/clob/client.rs (~line 276)
- Either widen to Err(_) or add doc comment explaining current behavior

### M8: Add Clone to request types
File: src/clob/types/request.rs
- Add Clone to all struct derive lists

### M9: Add Serialize to response types
File: src/clob/types/response.rs
- Add Serialize to all struct derive lists (requires serde::Serialize import)

### M10: Broaden retry error conditions
File: src/lib.rs (~line 106)
- Add error.is_request() to the retry condition

## Batch 4: LOW cleanup

### L1: Add downcast_ref to Error (src/error.rs)
### L2: Add error unit tests (src/error.rs)
### L3: Add PRIVATE_KEY_VAR constant (src/lib.rs)
### L5: Add rust_decimal_macros dep + dec! re-export (Cargo.toml, src/types.rs)
### L6: Remove blanket clippy allows (src/lib.rs:1-7), fix per-item
### L7: Add doc comments to clob/mod.rs, types.rs, auth.rs, error.rs
### L8: Create tests/common/mod.rs with shared helpers
### L12: Add comment about taker field omission (src/clob/client.rs)

## Rules
- Run cargo check && cargo test && cargo clippy -- -D warnings after each batch.
- Read the reference files BEFORE making changes.
- Do NOT change module structure or public API beyond what the fix requires.
- Update the progress log after each batch.
```

---

## 6. Progress Log

| Date | Batch | Status | Notes |
|---|---|---|---|
| 2026-04-19 | — | Plan drafted | 20-agent audit: 2 CRITICAL, 7 HIGH, 10 MEDIUM, 13 LOW findings |
| 2026-04-19 | Batch 1 | Completed | Fixed HMAC secret decoding to use standard base64 input while preserving URL-safe base64 output, added a regression test for secrets containing `+` and `/`, and added explicit serde renames for `OrderResponse` camelCase fields with a deserialization test. Updated the version-mismatch retry fixture to use the real API response shape. `cargo check && cargo test && cargo clippy -- -D warnings` pass. |
| 2026-04-19 | Batch 2 | Completed | Switched L1/L2 auth headers to lowercase hex addresses, added default reqwest headers plus gzip support, constrained HTTP retries to POST requests, added funder/signature-type validation during authentication, and fixed orderbook hashing compatibility by serializing `OrderSummary` decimals as strings. Added coverage for lowercase address headers, default `User-Agent`, POST-only retry behavior, and invalid auth configurations. `cargo check && cargo test && cargo clippy -- -D warnings` pass. |
| 2026-04-19 | Batch 3 | Completed | Adopted the V1 clippy baseline, removed the stray HMAC body quote normalization, added tick-size precision validation for limit prices plus explicit market-order amount semantics docs, widened `create_or_derive_api_key` fallback to match TS behavior on non-status failures, preserved the intentional `post_only: false` V2 behavior with comments, ensured response serialization coverage including manual `ApiKeysResponse` serialization, and broadened transient retry classification with `is_request()`. Added regression tests for limit-price precision and non-status create-or-derive fallback. `cargo check && cargo test && cargo clippy -- -D warnings` pass. |
| 2026-04-19 | Batch 4 | Completed | Added `Error::downcast_ref` and error module regression tests, restored `PRIVATE_KEY_VAR`, re-exported `dec!`, removed the crate-level clippy blanket in favor of targeted per-item allowances with reasons, documented auth/clob/types/error modules, added `tests/common/mod.rs` shared helpers, and documented the intentional omission of `taker` from the V2 POST payload. Full verification passes: `cargo check`, `cargo check --features ws`, `cargo test`, `cargo clippy -- -D warnings`, `cargo clippy --features ws -- -D warnings`, and `cargo doc --no-deps`. |
| | Batch 5 | Deferred | M6-M7, L4, L9-L11, L13 |
