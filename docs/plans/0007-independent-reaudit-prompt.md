# Plan 0007: Independent Re-Audit Prompt — Closure & Findings

**Status:** CLOSED
**Created:** 2026-04-20
**Closed:** 2026-04-20
**Predecessors:** [0005-audit-prompt.md](0005-audit-prompt.md) (CLOSED), [0006-independent-audit-findings.md](0006-independent-audit-findings.md)
**Purpose:** Preserve the re-audit prompt and record the adjudicated outcome of the second two-pass, 20-area independent audit of the current `rs-clob-client-v2` repository state.

---

## 1. Re-Audit Outcome

The re-audit was executed against current `HEAD` using the same two-pass, 20-area structure defined in this plan. The runtime still capped concurrent agent threads below 20, so the audit ran in parallel sub-batches, but all 20 requested scopes completed.

**Final scores:**
- Overall V1 convention fidelity: **92/100**
- Overall TS V2 parity: **80/100**
- Production recommendation: **NO-GO**

**Claim adjudication:**
- The repository's self-reported **all green build/test** claim is supported.
- The repository's self-reported **GO** claim is **not** supported.

**Reason for NO-GO:**
- Market-order validation still misses TS V2 min/max price bounds for user-supplied market prices.
- Order-version mismatch retry behavior still does not fully match TS V2 on non-2xx mismatch responses and unchanged-version refreshes.
- Several TS-public GET endpoints are still only exposed on the authenticated Rust client surface.
- `MarketDetails.fd.r` still serializes as a string instead of a numeric shorthand field.
- The main REST client's insecure local-dev escape hatch remains broader than the now-fixed builder and websocket validators.

---

## 2. Claim Adjudication

The prompt asked the audit to challenge six current-state claims. The re-audit outcome was:

1. `create_or_derive_api_key()` now matches the default TS behavior.
   - **Supported.** The fallback-after-create-failure behavior now matches the TS default flow closely enough.
2. Remote builder signing enforces `https://` by default and only allows `http://` via explicit local-dev opt-in.
   - **Supported.** `src/auth.rs:301-339` now enforces this correctly.
3. `ws` feature-gated builds work and websocket scheme validation is enforced at the connection boundary.
   - **Supported.** The feature matrix is green and `src/ws/connection.rs:14-23` now validates schemes correctly.
4. The test suite is fully hermetic and no test under `tests/` references `https://clob.polymarket.com`.
   - **Supported.** The re-audit found no matches under `tests/`.
5. Structured non-2xx payloads, retry conditions, pagination behavior, and shorthand market types now match TS V2 closely enough for parity.
   - **Rejected.** Structured payload preservation and the basic retry envelope improved, but parity still fails on market-order price validation, version-mismatch retries, public endpoint auth placement, and shorthand fee-rate serialization.
6. The README clearly states this is Bullpen.fi's unofficial Rust implementation intended for testing and early adoption, and directs users toward an eventual official Polymarket Rust V2 SDK.
   - **Supported.** README messaging now reflects this correctly.

---

## 3. Verification Captured During Re-Audit

**Verification baseline captured directly during the re-audit:**
- `cargo check` — PASS
- `cargo check --no-default-features` — PASS
- `cargo check --no-default-features --features ws` — PASS
- `cargo check --all-features` — PASS
- `cargo test` — PASS
- `cargo test --features ws` — PASS
- `cargo clippy --all-targets --all-features -- -D warnings` — PASS
- `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` — PASS
- `cargo test -- --list` — **107 tests**
- `cargo test --features ws -- --list` — **112 tests**
- `rg -n "https://clob\\.polymarket\\.com" tests` — **no matches**

---

## 4. Security Checklist

1. HMAC base64 decode/encode alphabets — **PASS**
2. `SecretString` is used for credentials and builder bearer tokens — **PASS**
3. Salt generation stays within the JS-safe 53-bit range — **PASS**
4. Remote builder signing defaults to `https://` and only permits explicit `http://` local-dev opt-in — **PASS**
5. Websocket config defaults to `wss://` and only permits explicit `ws://` local-dev opt-in — **PASS**
6. Main REST host validation limits insecure opt-in to `http://` only — **FAIL**
7. Signed request body handling is strict UTF-8 — **PASS**
8. Funder/signature-type validation remains enforced — **PASS**
9. Debug output redacts credentials/tokens — **PASS**
10. No `unsafe` blocks were found under `src/` — **PASS**

---

## 5. Agent Adjudication Matrix

This section records all 20 scoped review areas and the final adjudicated result from the re-audit.

1. `lib.rs + module structure` — **PASS WITH NOTES**
   - `src/lib.rs:32-49` and `src/lib.rs:64-137` still keep tracing only partially wired around `ToQueryParams` and the core `request()` path.
2. `error.rs` — **PASS WITH NOTES**
   - `src/error.rs:97`, `src/error.rs:141`, and `src/error.rs:308` add payload preservation and a `ws` conversion without breaking the core V1 error conventions.
3. `auth.rs` — **PASS WITH NOTES**
   - `src/auth.rs:149`, `219`, and `273` keep a broader public auth surface than V1, and `src/auth.rs:301-339` adds the new remote-builder validation path.
4. `client.rs architecture` — **PASS WITH NOTES**
   - `src/clob/client.rs:144-208`, `224-234`, and `449-467` intentionally keep a V2-specific client/config/cache/pagination design rather than a V1 mirror.
5. `order_builder.rs` — **PASS WITH NOTES**
   - `src/clob/order_builder.rs:60` and `463` preserve the PhantomData pattern and 53-bit salt masking, while intentionally keeping a V2-specific builder state.
6. `serde_helpers + config + types` — **PASS WITH NOTES**
   - `src/serde_helpers.rs:6-15` restores the tracing-gated warning-preserving path; `StringFromAny` from V1 remains absent.
7. `request/response types` — **PASS WITH NOTES**
   - `src/clob/types/response.rs:73-77`, `183-194`, and `src/clob/types/trade.rs:52-56` keep more V2-specific response wrappers and more stringly typed trade/notification surfaces than V1.
8. `WebSocket infrastructure` — **PASS WITH NOTES**
   - `src/ws/config.rs:21` and `src/ws/connection.rs:14-23` now enforce the intended scheme guardrails, but the WS layer remains intentionally minimal compared with V1.
9. `Cargo.toml + dependencies` — **PASS WITH NOTES**
   - The crate keeps its V2-only feature surface and a slightly narrower lint/dependency posture than V1 while preserving the required `reqwest`/`tracing` wiring.
10. `tests + examples` — **PASS WITH NOTES**
   - The suite is hermetic, but websocket coverage is still validation-only and the example surface remains narrower than V1.
11. `EIP-712 signing` — **PASS**
   - No parity issues found.
12. `HMAC + L1/L2 authentication` — **PASS WITH NOTES**
   - `src/auth.rs:208-210` and `243-246` still normalize auth-header addresses to lowercase hex; the Rust-only remote builder flow is additive rather than TS-comparable.
13. `order amount math` — **FAIL**
   - `src/clob/order_builder.rs:389` and `444-448` still miss TS V2 market-order min/max price validation.
14. `market price calculation` — **PASS**
   - No parity issues found.
15. `endpoint paths + HTTP methods` — **FAIL**
   - `src/clob/client.rs:1464`, `1659`, and `1677` still place public TS GETs on the authenticated Rust client surface.
16. `POST payloads` — **PASS WITH NOTES**
   - `src/clob/client.rs:1720-1788` matches TS V2 camelCase names, numeric `signatureType`, salt encoding, and expiration placement.
17. `type serialization + constants` — **FAIL**
   - `src/clob/types/market.rs:115-117` still serializes `MarketDetails.fd.r` as a string instead of a numeric shorthand field.
18. `client method logic + getSigner` — **FAIL**
   - `src/clob/client.rs:1698-1707` still mishandles version-mismatch retries relative to TS behavior.
19. `HTTP behavior + error handling` — **PASS WITH NOTES**
   - `src/lib.rs:79-129` now matches the TS retry envelope and preserves structured payloads much better, but Rust still returns `Err(Status)` instead of TS's result-object flow.
20. `security review` — **FAIL**
   - `src/clob/client.rs:847-850` still treats `allow_insecure` as a blanket bypass for the main REST host scheme check.

---

## 6. Confirmed Remaining Issues

### HIGH

#### H1: Market-order price bounds still do not match TS V2

**Files:** `src/clob/order_builder.rs:389`, `src/clob/order_builder.rs:444-448`

Rust market-order building only applies `validate_price(price)` and that helper still enforces `price > 0` only. TS V2 rejects market prices outside `[tick_size, 1 - tick_size]` before raw-amount math. As a result, Rust can still accept out-of-range user-supplied market prices such as `1.2` and build maker/taker amounts for them.

**Why it matters:** This is a real TS V2 parity break on a core trading path.

**Required fix:**
- Reuse the same min/max price check already present for limit orders.
- Add explicit tests for market-order prices below `tick_size` and above `1 - tick_size`.

#### H2: Version-mismatch retry handling is still not TS-safe

**Files:** `src/clob/client.rs:1698-1707`, `src/lib.rs:87-108`

The Rust retry path only runs after `submit().await?` returns `Ok(Value)`. If `/order` returns a non-2xx payload containing `order_version_mismatch`, `crate::request()` converts it into `Error::Status` first, so Rust never reaches the refresh-and-retry path. Rust also always resubmits once after any observed mismatch marker, while TS only retries when a forced version refresh actually changed the cached version.

**Why it matters:** Orders can fail to retry when TS would retry, and Rust can also issue one unnecessary duplicate submission.

**Required fix:**
- Inspect version-mismatch payloads before converting them into terminal Rust errors.
- Retry only if a forced `/version` refresh actually changed the cached version.
- Add regression coverage for both the non-2xx mismatch case and the unchanged-version case.

### MEDIUM

#### M1: Public TS GET endpoints are still unnecessarily auth-gated in Rust

**Files:** `src/clob/client.rs:1464-1491`, `src/clob/client.rs:1659-1695`

`builder_fees()`, `current_rewards()`, and `raw_rewards_for_market()` still live under `impl<K: Kind> Client<Authenticated<K>>` even though TS exposes them as public no-auth reads.

**Why it matters:** Rust users need an authenticated client to perform reads that TS users can perform from the public client surface.

**Required fix:**
- Move these methods to the unauthenticated/public client surface, or add parallel public wrappers.
- Keep the cache behavior unchanged if the methods move.

#### M2: `MarketDetails.fd.r` still serializes as a string instead of a number

**Files:** `src/clob/types/market.rs:115-117`, `tests/serde.rs:74-86`

`FeeDetails.rate` is deserialized with a custom decimal helper, but it still serializes through `rust_decimal`'s default JSON representation. The current serde regression test explicitly expects `"fd": { "r": "0.02", ... }`, while the TS shorthand market type defines `r?: number`.

**Why it matters:** This leaves a real shorthand wire-format mismatch on a public response model.

**Required fix:**
- Add a custom serializer for `FeeDetails.rate`, or wrap the field in a numeric-serde helper similar to `NumericTickSize`.
- Update serde tests to expect numeric output.

### LOW

#### L1: Main REST `allow_insecure` still accepts arbitrary non-HTTPS schemes

**File:** `src/clob/client.rs:847-850`

`Client::new(..., Config::builder().allow_insecure(true).build())` still accepts any non-`https` scheme instead of limiting the local-dev escape hatch to `http://`. This is now inconsistent with the stricter remote-builder validator in `src/auth.rs:301-309` and the websocket validator in `src/ws/connection.rs:14-23`.

**Why it matters:** The main REST transport remains less strict than the now-fixed builder and websocket surfaces.

**Required fix:**
- Replace the blanket bypass with explicit `(https) || (http && allow_insecure)` validation.
- Add tests for rejected `ftp://` or other invalid schemes.

---

## 7. Archived Prompt Claim Snapshot

The repository owner claims that the post-`0006` remediation state is now back to a production **GO** recommendation.

**Current self-reported verification baseline at HEAD:**
- `cargo check` — PASS
- `cargo check --no-default-features` — PASS
- `cargo check --no-default-features --features ws` — PASS
- `cargo check --all-features` — PASS
- `cargo test` — PASS
- `cargo test --features ws` — PASS
- `cargo clippy --all-targets --all-features -- -D warnings` — PASS
- `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` — PASS
- `cargo test -- --list` — **107 tests**
- `cargo test --features ws -- --list` — **112 tests**

**Important recent claims to verify, not assume:**
1. `create_or_derive_api_key()` now matches the default TS behavior, including fallback after create failures that materialize as error objects without keys.
2. Remote builder signing now enforces `https://` by default and only allows `http://` via an explicit local-dev insecure escape hatch.
3. `ws` feature-gated builds work correctly and WebSocket scheme validation is enforced at the connection boundary.
4. The test suite is fully hermetic and no test under `tests/` depends on `https://clob.polymarket.com`.
5. Structured non-2xx payloads, retry conditions, pagination behavior, and shorthand market types now match TS V2 closely enough for parity.
6. The README clearly states this is Bullpen.fi’s unofficial Rust implementation intended for testing and early adoption, and that users should migrate to an official Polymarket Rust V2 SDK if Polymarket releases one.

This prompt is intentionally adversarial: the new audit should try to disprove those claims if the code does not actually support them.

---

## 8. Original Re-Audit Prompt

Copy everything below this line and give it to a new agent session.

---

```text
# Task: Full Independent Re-Audit of rs-clob-client-v2

You are auditing a Rust crate that reimplements Polymarket's TypeScript CLOB client V2
in Rust, following the architectural patterns of Polymarket's official Rust V1 SDK.

## Repository

/Users/hongjunwu/Documents/Git/rs-clob-client-v2/

## Layout

src/                          # The implementation under audit
references/rs-clob-client/    # Polymarket's official Rust V1 SDK (STYLE GUIDE)
references/clob-client-v2/    # Polymarket's TypeScript V2 SDK (FEATURE SPEC)
tests/                        # Integration tests
examples/                     # Example programs
docs/plans/                   # Implementation and audit history (0001-0006)

## What this crate does

It is a Rust trading SDK for Polymarket's CLOB (Central Limit Order Book) V2 API.
It handles EIP-712 order signing, HMAC-SHA256 API authentication, order building
with decimal math, and REST endpoint coverage for the Polymarket trading API.

## Your task

Spawn 20 subagents across two passes to audit the implementation again from scratch.
Each agent must read our source code and the relevant reference files, then report findings.
Compile all results into a single consolidated report.

If the runtime does not allow 20 truly simultaneous subagents, preserve the same 20 scoped
review areas and the same two-pass structure, but execute them in sub-batches as needed.

Do not assume the previous remediation is correct. Verify current HEAD directly.

### Pass 1: Convention Audit (10 agents) — compare against Rust V1 SDK

Each agent compares a specific area of our code against the V1 Rust reference
to verify we still follow the same idiomatic patterns where appropriate.

Agent 1 — lib.rs + module structure
  OURS: src/lib.rs, src/clob/mod.rs
  V1:   references/rs-clob-client/src/lib.rs, src/clob/mod.rs
  CHECK: feature gates, pub use re-exports, request() handler, ToQueryParams,
         PRIVATE_KEY_VAR removal, doc comments, tracing feature shape, clippy allows

Agent 2 — error.rs
  OURS: src/error.rs
  V1:   references/rs-clob-client/src/error.rs
  CHECK: Kind enum variants, Error struct (backtrace, source), downcast_ref,
         Display impls, From impls, Status payload preservation, Synchronization wording

Agent 3 — auth.rs
  OURS: src/auth.rs
  V1:   references/rs-clob-client/src/auth.rs
  CHECK: State/sealed traits, Credentials (SecretString), L1 EIP-712 (ClobAuth
         domain, address encoding), L2 HMAC (message format, base64 alphabets),
         Builder auth, remote signer URL validation, header constant names

Agent 4 — client.rs architecture
  OURS: src/clob/client.rs
  V1:   references/rs-clob-client/src/clob/client.rs
  CHECK: Client<State> with Arc<Inner>, Config struct, AuthenticationBuilder
         (funder/sigtype validation), DashMap caches, default HTTP headers,
         pagination design, retry boundaries, state promotion, cache invalidation

Agent 5 — order_builder.rs
  OURS: src/clob/order_builder.rs
  V1:   references/rs-clob-client/src/clob/order_builder.rs
  CHECK: PhantomData type-state, builder fields, price/size validation,
         salt generation (53-bit mask), RoundConfig, amount calculation,
         getSigner/resolve_signer precedence and error propagation

Agent 6 — serde_helpers + config + types
  OURS: src/serde_helpers.rs, src/config.rs, src/types.rs
  V1:   references/rs-clob-client/src/serde_helpers.rs, src/types.rs
  CHECK: tracing-gated deserialize_with_warnings, feature-gated helpers,
         ContractConfig (phf_map), type re-exports (Address, Decimal, dec!)

Agent 7 — request/response types
  OURS: src/clob/types/request.rs, response.rs, book.rs, trade.rs, market.rs, builder.rs
  V1:   references/rs-clob-client/src/clob/types/request.rs, response.rs
  CHECK: Builder derives, Clone on requests, serde renames, OrderSummary string serialization,
         structured response shapes, Page<T>, camelCase wire compatibility

Agent 8 — WebSocket infrastructure
  OURS: src/ws/, src/clob/ws/
  V1:   references/rs-clob-client/src/ws/, src/clob/ws/
  CHECK: feature gates, Connection guardrails, allow_insecure semantics, public API surface,
         intentional scope reduction versus missing required behavior

Agent 9 — Cargo.toml + dependencies
  OURS: Cargo.toml
  V1:   references/rs-clob-client/Cargo.toml
  CHECK: dep versions, feature flags, lint config, reqwest features (gzip, rustls),
         optional tracing/serde_ignored shape, rationale for lint deviations from V1

Agent 10 — tests + examples
  OURS: tests/, examples/
  V1:   references/rs-clob-client/tests/, examples/
  CHECK: test file count, tests/common/mod.rs, httpmock patterns, hermeticity,
         ws coverage, example completeness, README accuracy

### Pass 2: Correctness Audit (10 agents) — compare against TypeScript V2 SDK

Each agent compares a specific area of our code against the TS V2 reference
to verify functional correctness and API parity.

Agent 11 — EIP-712 signing
  OURS: src/clob/types/order.rs, src/config.rs
  TS:   references/clob-client-v2/src/order-utils/model/ctfExchangeV2TypedData.ts,
        src/order-utils/exchangeOrderBuilderV2.ts, src/order-utils/model/orderDataV2.ts,
        src/config.ts
  CHECK: sol! Order struct (11 fields, no expiration in typed data), domain
         (name, version "2"), contract addresses (Polygon + Amoy), SignatureTypeV2 (0-3),
         signature encoding and hash parity

Agent 12 — HMAC + L1/L2 authentication
  OURS: src/auth.rs
  TS:   references/clob-client-v2/src/signing/hmac.ts, src/headers/index.ts,
        src/signing/eip712.ts, src/signing/constants.ts
  CHECK: HMAC message (timestamp+method+path+body), base64 decode/encode alphabets,
         ClobAuth EIP-712 struct, header names, address format, builder remote flow

Agent 13 — order amount math
  OURS: src/clob/order_builder.rs
  TS:   references/clob-client-v2/src/order-builder/helpers/getOrderRawAmounts.ts,
        getMarketOrderRawAmounts.ts, roundingConfig.ts
  CHECK: RoundConfig (4 tick sizes), limit BUY/SELL maker/taker mapping,
         market BUY/SELL amounts, rounding, fee adjustment, zero-price guards

Agent 14 — market price calculation
  OURS: src/clob/client.rs (calculate_market_price)
  TS:   references/clob-client-v2/src/order-builder/helpers/calculateBuyMarketPrice.ts,
        calculateSellMarketPrice.ts
  CHECK: BUY accumulates size*price, SELL accumulates size, book traversal order,
         FOK guard, FAK fallback

Agent 15 — endpoint paths + HTTP methods
  OURS: src/clob/client.rs
  TS:   references/clob-client-v2/src/endpoints.ts, src/client.ts
  CHECK: all endpoint paths, HTTP methods (GET/POST/DELETE), auth levels
         (none/L1/L2), query params vs body, builder/rewards/scoring endpoints

Agent 16 — POST payloads
  OURS: src/clob/client.rs, src/clob/types/*
  TS:   references/clob-client-v2/src/types/ordersV2.ts, src/client.ts
  CHECK: field renames (tokenId, makerAmount, etc.), Side encoding ("BUY"/"SELL"),
         SignatureTypeV2 encoding (numbers 0-3), salt as JS-safe number, taker omission,
         expiration placement (payload vs typed data)

Agent 17 — type serialization + constants
  OURS: src/clob/types/ (all files), src/config.rs
  TS:   references/clob-client-v2/src/types/clob.ts, src/config.ts, src/constants.ts
  CHECK: OrderResponse camelCase renames, OrderSummary string serialization,
         MarketDetails shorthand fields, enum values, contract addresses, tick-size representation

Agent 18 — client method logic + getSigner
  OURS: src/clob/client.rs, src/clob/order_builder.rs
  TS:   references/clob-client-v2/src/client.ts, src/order-builder/orderBuilder.ts
  CHECK: createOrDeriveApiKey fallback semantics, version caching, version-mismatch retry,
         useServerTime, getSigner (factory precedence, order-signing-only scope)

Agent 19 — HTTP behavior + error handling
  OURS: src/lib.rs, src/clob/client.rs, src/error.rs
  TS:   references/clob-client-v2/src/http-helpers/index.ts, src/errors.ts
  CHECK: retry (POST-only, 30ms, 1 retry), retry conditions, default headers
         (User-Agent, Accept, Connection, gzip), structured non-2xx handling,
         body preservation and error-message derivation

Agent 20 — security review
  OURS: src/ (all files), Cargo.toml
  CHECK: HMAC base64 alphabets, SecretString on credentials, salt masking,
         HTTPS/WSS enforcement, insecure local-dev escape hatches, strict UTF-8,
         address encoding, funder/sigtype validation, no unsafe blocks, no credential leakage

## Required verification

Run these yourself as part of the audit:

- `cargo check`
- `cargo check --no-default-features`
- `cargo check --no-default-features --features ws`
- `cargo check --all-features`
- `cargo test`
- `cargo test --features ws`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps`
- `cargo test -- --list`
- `cargo test --features ws -- --list`

Also explicitly check whether any file under `tests/` still references:

- `https://clob.polymarket.com`

## Report format

For each agent, use this format:

  AREA: [name]
  VERDICT: PASS / PASS WITH NOTES / FAIL
  FINDINGS: [bullet list of issues, if any, with file:line references]

After all 20 agents complete, compile a consolidated report with:

1. Overall V1 Convention Fidelity Score (out of 100)
2. Overall TS V2 Parity Score (out of 100)
3. Security Checklist (10 items, PASS/FAIL)
4. Build Verification (commands above, run by you)
5. Total test count (default and `ws`)
6. GO / NO-GO production recommendation
7. Ranked list of any remaining issues by severity (CRITICAL/HIGH/MEDIUM/LOW)
8. Explicit statement whether the repo’s self-reported “all green / GO” claim is supported

## Context from prior audits

Plans 0001-0006 document the implementation and earlier audit history.

The previous 20-area audit (0005 -> 0006) produced:
- Overall V1 convention fidelity: 84/100
- Overall TS V2 parity: 90/100
- Production recommendation: NO-GO

The repository owner claims all of those actionable issues were remediated in the current state.
Two additional follow-up fixes were then applied after that remediation:

1. `create_or_derive_api_key()` was updated again to align with the TS default flow and fall back to derive after failed create attempts.
2. `remote_insecure()` was narrowed so it only accepts `http://` for local development, instead of allowing arbitrary non-HTTPS schemes.

Known intentional differences that may still be acceptable if well-justified:
- WebSocket support remains minimal and feature-gated rather than a full V1-style implementation
- Tracing exists only for serde warning diagnostics, not as broad request instrumentation
- No V1 wallet-derivation helpers (`derive_proxy_wallet`, `derive_safe_wallet`)
- Auth headers use lowercase hex addresses, while order payloads follow the expected payload conventions
- This is an unofficial Bullpen.fi implementation; README should direct users to an official Polymarket Rust V2 SDK if one is released

Your job is to independently verify or reject those claims.
```

---

## Notes

This prompt is designed to be copy-pasted into a fresh agent session with no prior context. The agent should:
1. Read the prompt
2. Spawn 20 scoped audit agents across the two passes
3. Run the required verification commands directly
4. Produce a consolidated GO / NO-GO recommendation

The prompt references exact files and reference paths so auditors can do targeted reads without wandering the tree first.
