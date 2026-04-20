# Plan 0006: Independent Audit Findings — Closure & Remediation

**Status:** Ready
**Created:** 2026-04-20
**Predecessor:** [0005-audit-prompt.md](0005-audit-prompt.md) (CLOSED)
**Goal:** Convert the independent audit into a single adjudicated findings log, separate confirmed issues from rejected or accepted drift items, and define the remediation order required to restore a GO recommendation.

---

## 1. Audit Outcome

The independent audit was executed across 20 scoped review areas. Because the agent runtime only allowed 6 concurrent subagents, the audit ran in parallel sub-batches rather than all 20 at once, but all requested scopes completed.

**Final scores:**
- Overall V1 convention fidelity: **84/100**
- Overall TS V2 parity: **90/100**
- Production recommendation: **NO-GO**

**Verification baseline captured during the audit:**
- `cargo check` — PASS
- `cargo test` — PASS
- `cargo test -- --list` — **100 tests total**
- `cargo clippy --all-targets --all-features -- -D warnings` — FAIL
- `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` — PASS
- `cargo check --no-default-features --features ws` — FAIL

**Reason for NO-GO:**
- The crate does not currently satisfy its advertised feature-gated build surface.
- Security-sensitive remote signer configuration does not enforce TLS.
- One client fallback path masks real failures and diverges from TS V2 behavior.
- A few response and shorthand model shapes still do not match TS V2.
- The test suite is not fully hermetic, and the clippy gate is red.

---

## 2. Agent Adjudication Matrix

This section records every area that was audited, including findings that were later accepted as intentional or rejected after spot-checking.

| Agent | Area | Raw Verdict | Adjudicated Outcome |
|---|---|---|---|
| 1 | `lib.rs + module structure` | PASS WITH NOTES | Confirmed a stale module doc comment in `src/clob/mod.rs:1-4`; no functional issue. |
| 2 | `error.rs` | FAIL | Confirmed minor V1 convention drift in `Synchronization` display text and the extra WebSocket `From` impl; non-blocking. |
| 3 | `auth.rs` | FAIL | Rejected as a current bug. The V1-style HMAC/body-string concern does not apply to TS V2 parity, and TS auth vectors passed. |
| 4 | `client.rs architecture` | FAIL | Mixed result. Confirmed `User-Agent` mismatch and `create_or_derive_api_key` fallback drift. Rejected the stale version-cache claim and the initial-cursor claim. Accepted config-shape drift as intentional V2 evolution. |
| 5 | `order_builder.rs` | FAIL | Accepted as intentional V2 drift after parity spot-check. TS order math passed. |
| 6 | `serde_helpers + config + types` | FAIL | Confirmed the `ws`-only build break in `src/serde_helpers.rs:5` and the removal of the warning-preserving deserialize path. Accepted missing V1 wallet-config coverage as V1-only scope. |
| 7 | `request/response types` | FAIL | Rejected after spot-check. The reported request-clone and drop-notification concerns did not hold up as wire-format bugs. |
| 8 | `WebSocket infrastructure` | PASS WITH NOTES | Confirmed `allow_insecure` is effectively dead in `ws::Config::parse`, and the public builder path still needs explicit scheme enforcement. |
| 9 | `Cargo.toml + dependencies` | FAIL | Confirmed lint-policy drift and minor dependency/version drift from V1; non-blocking by itself, but related to the clippy failure. |
| 10 | `tests + examples` | FAIL | Confirmed live-host integration tests, missing WebSocket coverage, and sparse examples relative to the exposed surface. |
| 11 | `EIP-712 signing` | PASS | Confirmed parity. |
| 12 | `HMAC + L1/L2 authentication` | PASS | Confirmed parity. |
| 13 | `order amount math` | PASS | Confirmed parity. |
| 14 | `market price calculation` | PASS | Confirmed parity. |
| 15 | `endpoint paths + HTTP methods` | PASS | Confirmed parity. |
| 16 | `POST payloads` | PASS | Confirmed parity. |
| 17 | `type serialization + constants` | FAIL | Confirmed `MarketDetails` shorthand shape/type mismatches in `src/clob/types/market.rs:37-54`. |
| 18 | `client method logic + getSigner` | FAIL | Confirmed over-broad create/derive fallback and the extra `MAX_PAGES` cap. |
| 19 | `HTTP behavior + error handling` | FAIL | Confirmed `User-Agent` drift, broader retry conditions, and flattening of structured non-2xx payloads. |
| 20 | `security review` | FAIL | Confirmed missing HTTPS enforcement for remote builder config and missing public-path WSS enforcement for WebSocket config. |

---

## 3. Confirmed Actionable Findings

### HIGH

#### H1: `ws`-only feature build is broken

**Files:** `src/serde_helpers.rs:5`, `src/lib.rs:4-11`

**Problem:** `serde_helpers.rs` unconditionally imports `crate::clob::types::TickSize`, but the `clob` module is gated behind `feature = "clob"`. As a result, `cargo check --no-default-features --features ws` fails with `could not find clob in the crate root`.

**Why it matters:** This is a real crate-surface break, not just a convention issue. The advertised feature matrix is invalid.

**Required fix:**
- Gate `deserialize_tick_size` and the `TickSize` import behind `#[cfg(feature = "clob")]`, or move tick-size helpers into the `clob` module.
- Add an explicit feature-matrix check to CI:
  - `cargo check --no-default-features`
  - `cargo check --no-default-features --features ws`
  - `cargo check --all-features`

**Verification:**
- `cargo check --no-default-features --features ws`

#### H2: Remote builder signer configuration does not enforce HTTPS

**Files:** `src/auth.rs:300-305`, `src/auth.rs:356-380`

**Problem:** `auth::builder::Config::remote()` accepts any URL scheme. `Builder::create_headers()` then POSTs the signing payload and optional bearer token to that host without verifying that the scheme is HTTPS.

**Why it matters:** This can leak builder bearer tokens and signing requests over plaintext transport.

**Required fix:**
- Reject non-HTTPS URLs in `Config::remote()` unless there is an explicit local-dev escape hatch.
- If an escape hatch is kept, it must be opt-in and mirrored in `create_headers()`.
- Add tests for both secure and rejected insecure hosts.

**Verification:**
- Unit tests for `Config::remote("https://...")` and `Config::remote("http://...")`
- Integration test for header creation against a secure mock endpoint

#### H3: WebSocket scheme enforcement is inconsistent and bypassable

**Files:** `src/ws/config.rs:13-34`, `src/ws/connection.rs:15-17`

**Problem:** `ws::Config::parse()` rejects non-`wss` URLs, but the public `Builder`-derived constructor path can still create a `Config` with an arbitrary `url`, and `Connection::connect()` does not revalidate either the scheme or `allow_insecure`.

**Why it matters:** Users can establish insecure WebSocket connections even though the parser appears to prohibit them. The current `allow_insecure` flag is also misleading because the parse helper never honors it.

**Required fix:**
- Enforce scheme policy in `Connection::connect()` as the final guard.
- Either remove `allow_insecure` from `ws::Config` for now or make it functional and explicit.
- Add tests covering both parse-time and connect-time validation.

**Verification:**
- `cargo check --all-features`
- New tests for `ws://` and `wss://` behavior

#### H4: `create_or_derive_api_key` masks real create failures

**File:** `src/clob/client.rs:323-331`

**Problem:** Rust falls back from `create_api_key()` to `derive_api_key()` on any create error. The TS V2 client only falls back when `createApiKey()` returns a response without a usable key.

**Why it matters:** Transport errors, HTTP failures, or internal bugs can be silently hidden by the derive fallback, making failures harder to detect and changing client semantics.

**Required fix:**
- Narrow the fallback to match TS V2 behavior.
- Preserve the original create failure unless the returned create response is semantically empty.
- Update the existing fallback test to assert the new intended behavior.

**Verification:**
- Integration tests for:
  - successful create
  - create returns empty key then derive succeeds
  - create fails and the failure propagates

#### H5: `MarketDetails` shorthand fields do not match TS V2

**Files:** `src/clob/types/market.rs:37-54`

**Problem:**
- `t` is modeled as `Vec<Option<ClobToken>>`, but TS defines a fixed two-element tuple.
- `mts` is modeled as `TickSize`, which serializes as a string-form enum value, while TS defines a numeric field.
- `mbf` and `tbf` are modeled as `Option<String>`, while TS defines optional numbers.

**Why it matters:** This is a real response-shape mismatch on a public API type.

**Required fix:**
- Change `t` to a fixed two-element representation.
- Make `mts` a numeric type or a wrapper that serializes numerically.
- Change `mbf` and `tbf` to numeric types.
- Expand serde tests specifically around the shorthand market-details payload.

**Verification:**
- `tests/serde.rs`
- Round-trip tests using TS-shaped JSON fixtures

#### H6: Tests are not fully hermetic

**Files:** `tests/common/mod.rs:24-34`, `tests/order.rs:12-40`, `tests/get_signer.rs:22-31`, `tests/order_amounts.rs:14-24`

**Problem:** Several tests still authenticate a client against `https://clob.polymarket.com` directly. They rely on local cache injection to avoid most network usage, but they are still not hermetic.

**Why it matters:** CI reliability depends on external service stability, network availability, and production endpoint behavior.

**Required fix:**
- Replace live-host usage with `httpmock` or a local helper host.
- Split pure order-building tests from network/client tests so the former never touch HTTP configuration.
- Add explicit WebSocket tests if the `ws` surface remains public.

**Verification:**
- Test suite passes with all network egress blocked except local mock servers

#### H7: The clippy quality gate is red

**Files:** `tests/client.rs:45`, `examples/basic.rs:6-7`, `examples/market_data.rs:10-12`, `examples/orders.rs:1,30`, `tests/signing.rs:27-35`

**Problem:** `cargo clippy --all-targets --all-features -- -D warnings` fails on unreadable literals, `println!` in examples, anonymous trait imports, and an exhaustive `sol!`-generated test struct.

**Why it matters:** The audit explicitly required build verification, and the clippy step is currently failing.

**Required fix:**
- Normalize literals with separators.
- Add targeted `#[allow(...)]` where example output is intentional.
- Convert `use std::str::FromStr;` to `use std::str::FromStr as _;` where needed.
- Add the appropriate `#[allow(clippy::exhaustive_structs)]` around the `sol!` test helper if that is the intended style.

**Verification:**
- `cargo clippy --all-targets --all-features -- -D warnings`

---

### MEDIUM

#### M1: Structured non-2xx server payloads are flattened into strings

**Files:** `src/lib.rs:75-96`, `src/error.rs:78-90`

**Problem:** Non-success responses are converted into a plain `Error::status(status, method, path, message)` string. TS preserves structured error payloads and status fields via its `ApiError` model.

**Why it matters:** Rust callers lose structured server error context that exists in TS V2.

**Required fix:**
- Extend `Status` to optionally carry structured JSON or the original raw body.
- Preserve the response body shape instead of collapsing it immediately into text.
- Add regression tests for structured error payloads.

#### M2: Retry scope is broader than TS V2

**File:** `src/lib.rs:98-107`

**Problem:** Rust retries `reqwest::Error::is_request()` in addition to connect and timeout errors. TS only retries POSTs on no-response/network errors, 5xx, and a small set of timeout/network codes.

**Why it matters:** Behavior diverges from TS V2, especially for malformed request or protocol-level failures.

**Required fix:**
- Narrow retry conditions to the closest `reqwest` equivalents of TS V2.
- Keep the existing POST-only restriction.
- Add tests proving GET/DELETE still do not retry and malformed request errors do not retry.

#### M3: Default `User-Agent` does not match TS V2

**File:** `src/clob/client.rs:792-799`

**Problem:** Rust sends `polymarket-clob-client-v2`, while TS sends `@polymarket/clob-client`.

**Why it matters:** This is a parity mismatch in the default HTTP fingerprint.

**Required fix:**
- Decide whether Rust should intentionally identify itself differently.
- If parity is the priority, change the header to match TS.
- Document the choice either way.

#### M4: Pagination adds an extra hard cap not present in TS

**File:** `src/clob/client.rs:395-402`

**Problem:** `collect_pages()` aborts after 1000 pages. TS loops until `END_CURSOR` without a hard cap.

**Why it matters:** Very large result sets can fail in Rust while succeeding in TS.

**Required fix:**
- Either remove the cap or make it configurable.
- If the cap is retained for safety, document the intentional divergence and surface a more specific error type.

#### M5: `deserialize_with_warnings` no longer preserves unknown-field diagnostics

**File:** `src/serde_helpers.rs:68-69`

**Problem:** The helper now reduces to a plain `serde_json::from_value`, unlike the V1 warning-preserving path.

**Why it matters:** This weakens observability when Polymarket adds or changes fields.

**Required fix:**
- Restore the warning-preserving behavior, or at minimum gate it behind a feature.
- Add a test that proves unknown fields can be observed in debug/tracing mode.

#### M6: Example and WebSocket coverage is still thin

**Files:** `examples/basic.rs`, `examples/market_data.rs`, `examples/orders.rs`, `src/clob/ws/mod.rs`

**Problem:** The crate exposes a `ws` surface but ships no dedicated WebSocket tests or examples. The examples also trip clippy.

**Why it matters:** The public surface is ahead of its documentation and verification.

**Required fix:**
- Add one minimal `ws` example or hide the module until it is better supported.
- Add at least one smoke test for the `ws` feature.
- Clean up the existing examples so they pass clippy.

---

### LOW

#### L1: `src/clob/mod.rs` doc comment does not reflect feature gating

**Files:** `src/clob/mod.rs:1-4`, `src/lib.rs:4-11`

**Problem:** The module docs imply unconditional availability even though `clob` is feature-gated at the crate root.

**Required fix:** Update the module-level docs to mention the feature gate.

#### L2: Minor `error.rs` convention drift from the V1 SDK

**Files:** `src/error.rs:155-160`, `src/error.rs:283-287`

**Problem:** `Synchronization` display text differs from V1, and Rust V2 adds a WebSocket `From` impl that V1 did not expose.

**Required fix:** None required for correctness. Decide whether exact V1 surface fidelity still matters here.

#### L3: Cargo lint/dependency drift from the V1 style guide

**Files:** `Cargo.toml:34`, `Cargo.toml:57-61`

**Problem:** `futures` trails the V1 pin by one patch, `allow_attributes` was relaxed, and `same_name_method` from the V1 lint block is missing.

**Required fix:** Align with V1 where that still reflects the project’s standards, then rerun clippy.

#### L4: `ws::Config::allow_insecure` is currently misleading

**Files:** `src/ws/config.rs:15-33`

**Problem:** The field exists, but `parse()` always rejects non-`wss` URLs and never consults it.

**Required fix:** Either remove the field for now or make it functional and documented.

---

## 4. Spotted But Rejected or Accepted

These items were surfaced during the audit but were not retained as current bugs after manual spot-checking.

### Rejected After Verification

1. **Auth/HMAC parity bug in `src/auth.rs:411-425`**
   - Rejected.
   - The current implementation uses `STANDARD` for base64 secret decoding and `URL_SAFE` for output encoding, which matches TS V2 behavior.
   - TS auth parity tests passed.

2. **`retry_order_submission()` reuses a stale cached version**
   - Rejected.
   - `src/clob/client.rs:1620-1623` explicitly invalidates the cache before refetching `version()`.

3. **Pagination should not start with `MA==`**
   - Rejected.
   - TS V2 defines `INITIAL_CURSOR = "MA=="` in `references/clob-client-v2/src/constants.ts:6`.

4. **`DropNotificationsRequest` wire format is wrong**
   - Rejected.
   - `src/clob/client.rs:1136-1154` joins IDs into the comma-separated query-string format that TS uses.

5. **Missing `Clone` on request types listed by the audit**
   - Rejected.
   - The specifically cited request types already derive `Clone` in `src/clob/types/request.rs`.

### Accepted Intentional Drift

1. **No V1 wallet derivation helpers**
   - Accepted.
   - This matches the previously documented V2 scope decision.

2. **No tracing feature**
   - Accepted for now.
   - Still worth revisiting later, but not treated as a newly discovered bug.

3. **`order_builder.rs` shape drift from the V1 `Amount` wrapper / salt-generator injection**
   - Accepted.
   - TS V2 amount math passed, so this remains a design divergence rather than a parity defect.

4. **`Client::Config` shape drift from the V1 SDK**
   - Accepted.
   - The added Rust V2 fields (`retry_on_error`, `allow_insecure`, `signer_factory`) are deliberate V2-facing API choices.

5. **`error.rs` extra WebSocket conversion surface**
   - Accepted as non-blocking.
   - This is a convention drift, not a correctness issue.

---

## 5. Remediation Order

### Batch 1: Restore release integrity

1. Fix the `ws`-only feature build break in `serde_helpers.rs`.
2. Enforce HTTPS for remote builder config.
3. Enforce WSS or an explicit opt-in insecure mode at WebSocket connect time.
4. Fix `create_or_derive_api_key` fallback semantics.

### Batch 2: Restore TS V2 parity gaps

5. Correct `MarketDetails` shorthand field modeling.
6. Preserve structured non-2xx error payloads.
7. Narrow retry conditions to TS-equivalent behavior.
8. Decide and document the `User-Agent` strategy.
9. Revisit the hard `MAX_PAGES` cap and either remove, configure, or document it.

### Batch 3: Restore QA quality gates

10. Make the test suite hermetic by removing live-host dependencies.
11. Add minimal `ws` coverage or reduce the public `ws` surface.
12. Fix all clippy failures across tests and examples.

### Batch 4: Cleanup convention drift

13. Restore unknown-field warning behavior or add a feature-gated equivalent.
14. Update stale docs and minor V1-style inconsistencies.
15. Decide whether to realign the Cargo lint block with the V1 style guide.

---

## 6. Verification Checklist

This plan is complete only when all of the following pass:

- [x] `cargo check`
- [x] `cargo check --no-default-features`
- [x] `cargo check --no-default-features --features ws`
- [x] `cargo check --all-features`
- [x] `cargo test`
- [x] `cargo clippy --all-targets --all-features -- -D warnings`
- [x] `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps`
- [x] No test in `tests/` depends on `https://clob.polymarket.com`
- [x] Public `ws` behavior is either fully verified or explicitly reduced in scope
- [x] The production recommendation can be upgraded from NO-GO to GO

---

## 7. Progress Log

| Date | Batch | Status | Notes |
|---|---|---|---|
| 2026-04-20 | Batch 1 | DONE | Fixed `ws`-only feature gating in `serde_helpers`, enforced HTTPS-by-default for remote builder signing with explicit `remote_insecure` local-dev opt-in, added final WebSocket scheme validation in `Connection::connect`, aligned `create_or_derive_api_key` fallback semantics with TS V2, and verified with `cargo check`, `cargo test`, `cargo clippy -- -D warnings`, `cargo check --no-default-features`, `cargo check --no-default-features --features ws`, `cargo check --all-features`, and `cargo test --features ws`. |
| 2026-04-20 | Batch 2 | DONE | Realigned `MarketDetails` to the TS shorthand schema with fixed token slots, numeric min tick size, and numeric base fees; preserved structured non-2xx payloads on `error::Status`; narrowed request retries to connect/timeout failures; matched the TS default `User-Agent`; removed the extra Rust-only pagination cap; and verified with `cargo check`, `cargo test`, and `cargo clippy -- -D warnings`. |
| 2026-04-20 | Batch 3 | DONE | Removed remaining live-host dependencies from `tests/` by switching pure order/signing coverage to a local-safe placeholder host, kept explicit `ws` validation coverage in the `ws` module tests, fixed the all-targets/all-features clippy failures across tests and examples, and verified with `cargo check`, `cargo test`, `cargo clippy -- -D warnings`, and `cargo clippy --all-targets --all-features -- -D warnings`. |
| 2026-04-20 | Batch 4 | DONE | Restored the V1-style tracing-gated unknown-field warning path in `serde_helpers`, aligned the synchronization error wording with the V1 SDK, and realigned the clippy lint block to the V1 baseline except for the documented `allow_attributes*` relaxation required by `bon`/`sol!`-generated internal attributes. Final verification passed with `cargo check`, `cargo check --no-default-features`, `cargo check --no-default-features --features ws`, `cargo check --all-features`, `cargo test`, `cargo test --features ws`, `cargo clippy --all-targets --all-features -- -D warnings`, and `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps`. |
