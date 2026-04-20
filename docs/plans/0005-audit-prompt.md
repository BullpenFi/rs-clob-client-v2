# Plan 0005: Independent Audit Prompt

**Status:** CLOSED
**Created:** 2026-04-20
**Closed:** 2026-04-20
**Purpose:** Archive the self-contained prompt that was used to independently audit the rs-clob-client-v2 implementation against both reference repositories.

---

## Closure Summary

This prompt was executed and the audit is complete.

**Consolidated outcome:**
- Overall V1 convention fidelity score: **84/100**
- Overall TS V2 parity score: **90/100**
- Total test count observed during verification: **100**
- Production recommendation: **NO-GO**

**Build verification:**
- `cargo check` — PASS
- `cargo test` — PASS
- `cargo clippy --all-targets --all-features -- -D warnings` — FAIL
- `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` — PASS
- `cargo check --no-default-features --features ws` — FAIL

**Most important confirmed findings:**
1. `src/serde_helpers.rs:5` breaks `ws`-only builds because it unconditionally imports `crate::clob::types::TickSize`.
2. `src/auth.rs:300-305` allows non-HTTPS remote builder hosts, which can leak bearer tokens.
3. `src/clob/client.rs:323-331` falls back from `create_api_key` to `derive_api_key` on any create failure, which diverges from TS V2 behavior and can hide real errors.
4. `src/clob/types/market.rs:37-54` models `MarketDetails` shorthand fields with TS-incompatible shapes and scalar types.
5. Several tests still hit `https://clob.polymarket.com` directly instead of staying hermetic.

The full adjudicated findings log and remediation plan lives in [0006-independent-audit-findings.md](0006-independent-audit-findings.md).

---

## Audit Prompt

Copy everything below this line and give it to a new agent session.

---

```
# Task: Full Independent Audit of rs-clob-client-v2

You are auditing a Rust crate that reimplements Polymarket's TypeScript CLOB client V2
in Rust, following the architectural patterns of Polymarket's official Rust V1 SDK.

## Repository

/Users/hongjunwu/Documents/Git/rs-clob-client-v2/

## Layout

src/                          # The implementation under audit
references/rs-clob-client/    # Polymarket's official Rust V1 SDK (STYLE GUIDE)
references/clob-client-v2/    # Polymarket's TypeScript V2 SDK (FEATURE SPEC)
tests/                        # Integration tests (10 files, ~100 tests)
docs/plans/                   # Implementation history (Plans 0001-0004)

## What this crate does

It is a Rust trading SDK for Polymarket's CLOB (Central Limit Order Book) V2 API.
It handles EIP-712 order signing, HMAC-SHA256 API authentication, order building
with decimal math, and REST endpoint coverage for the full Polymarket trading API.

## Your task

Spawn 20 subagents in parallel across two passes to audit the implementation.
Each agent should read our source code AND the relevant reference files, then
report findings. Compile all results into a single consolidated report.

### Pass 1: Convention Audit (10 agents) — compare against Rust V1 SDK

Each agent compares a specific area of our code against the V1 Rust reference
to verify we follow the same idiomatic patterns.

Agent 1 — lib.rs + module structure
  OURS: src/lib.rs, src/clob/mod.rs
  V1:   references/rs-clob-client/src/lib.rs, src/clob/mod.rs
  CHECK: feature gates, pub use re-exports, request() handler, ToQueryParams,
         PRIVATE_KEY_VAR, doc comments, clippy allows

Agent 2 — error.rs
  OURS: src/error.rs
  V1:   references/rs-clob-client/src/error.rs
  CHECK: Kind enum variants, Error struct (backtrace, source), downcast_ref,
         Display impls, From impls, unit tests

Agent 3 — auth.rs
  OURS: src/auth.rs
  V1:   references/rs-clob-client/src/auth.rs
  CHECK: State/sealed traits, Credentials (SecretString), L1 EIP-712 (ClobAuth
         domain, address encoding), L2 HMAC (message format, base64 alphabets),
         Builder auth, header constant names

Agent 4 — client.rs architecture
  OURS: src/clob/client.rs
  V1:   references/rs-clob-client/src/clob/client.rs
  CHECK: Client<State> with Arc<Inner>, Config struct, AuthenticationBuilder
         (funder/sigtype validation), DashMap caches, default HTTP headers,
         request handler, pagination, retry logic

Agent 5 — order_builder.rs
  OURS: src/clob/order_builder.rs
  V1:   references/rs-clob-client/src/clob/order_builder.rs
  CHECK: PhantomData type-state, builder fields, price/size validation,
         salt generation (53-bit mask), RoundConfig, amount calculation,
         getSigner/resolve_signer, Amount type

Agent 6 — serde_helpers + config + types
  OURS: src/serde_helpers.rs, src/config.rs, src/types.rs
  V1:   references/rs-clob-client/src/serde_helpers.rs, src/lib.rs (config), src/types.rs
  CHECK: StringFromAny, deserialize_with_warnings, ContractConfig (phf_map),
         type re-exports (Address, Decimal, dec!)

Agent 7 — request/response types
  OURS: src/clob/types/request.rs, response.rs, book.rs, trade.rs, market.rs, builder.rs
  V1:   references/rs-clob-client/src/clob/types/request.rs, response.rs
  CHECK: Builder derives, Clone on requests, Serialize on responses,
         OrderSummary string serialization, serde renames, Page<T>

Agent 8 — WebSocket infrastructure
  OURS: src/ws/, src/clob/ws/
  V1:   references/rs-clob-client/src/ws/, src/clob/ws/
  CHECK: Connection struct, reconnection logic, MessageParser trait,
         broadcast channels, subscription management, feature gates

Agent 9 — Cargo.toml + dependencies
  OURS: Cargo.toml
  V1:   references/rs-clob-client/Cargo.toml
  CHECK: dep versions, feature flags, lint config (59 rules), reqwest features
         (gzip, rustls), alloy features, dev-dependencies

Agent 10 — tests + examples
  OURS: tests/, examples/
  V1:   references/rs-clob-client/tests/, examples/
  CHECK: test file count, tests/common/mod.rs, httpmock patterns, test coverage
         gaps, example completeness

### Pass 2: Correctness Audit (10 agents) — compare against TypeScript V2 SDK

Each agent compares a specific area of our code against the TS V2 reference
to verify functional correctness and API parity.

Agent 11 — EIP-712 signing
  OURS: src/clob/types/order.rs, src/config.rs
  TS:   references/clob-client-v2/src/order-utils/model/ctfExchangeV2TypedData.ts,
        src/order-utils/exchangeOrderBuilderV2.ts, src/order-utils/model/orderDataV2.ts,
        src/config.ts
  CHECK: sol! Order struct (11 fields, no expiration), domain (name, version "2"),
         contract addresses (Polygon + Amoy), SignatureTypeV2 (0-3), signature encoding

Agent 12 — HMAC + L1/L2 authentication
  OURS: src/auth.rs
  TS:   references/clob-client-v2/src/signing/hmac.ts, src/headers/index.ts,
        src/signing/eip712.ts, src/signing/constants.ts
  CHECK: HMAC message (timestamp+method+path+body), base64 (STANDARD decode,
         URL_SAFE encode), ClobAuth EIP-712 struct, header names, address format

Agent 13 — order amount math
  OURS: src/clob/order_builder.rs
  TS:   references/clob-client-v2/src/order-builder/helpers/getOrderRawAmounts.ts,
        getMarketOrderRawAmounts.ts, roundingConfig.ts
  CHECK: RoundConfig (4 tick sizes), limit BUY/SELL maker/taker mapping,
         market BUY/SELL amounts, rounding (roundNormal/Down/Up), fee adjustment

Agent 14 — market price calculation
  OURS: src/clob/client.rs (calculate_market_price)
  TS:   references/clob-client-v2/src/order-builder/helpers/calculateBuyMarketPrice.ts,
        calculateSellMarketPrice.ts
  CHECK: BUY accumulates size*price, SELL accumulates size, reverse iteration,
         FOK guard, FAK fallback

Agent 15 — endpoint paths + HTTP methods
  OURS: src/clob/client.rs
  TS:   references/clob-client-v2/src/endpoints.ts, src/client.ts
  CHECK: all ~60 endpoint paths, HTTP methods (GET/POST/DELETE), auth levels
         (none/L1/L2), query params vs body

Agent 16 — POST payloads
  OURS: src/clob/client.rs (PostOrderEnvelope, cancel payloads)
  TS:   references/clob-client-v2/src/types/ordersV2.ts, src/client.ts
  CHECK: field renames (tokenId, makerAmount, etc.), Side encoding ("BUY"/"SELL"),
         SignatureTypeV2 encoding (numbers 0-3), salt as u64, taker omission

Agent 17 — type serialization + constants
  OURS: src/clob/types/ (all files), src/config.rs
  TS:   references/clob-client-v2/src/types/clob.ts, src/config.ts, src/constants.ts
  CHECK: OrderResponse camelCase renames, OrderSummary string serialization,
         MarketDetails shorthand fields, enum values, contract addresses

Agent 18 — client method logic + getSigner
  OURS: src/clob/client.rs, src/clob/order_builder.rs
  TS:   references/clob-client-v2/src/client.ts, src/order-builder/orderBuilder.ts
  CHECK: createOrDeriveApiKey fallback, version caching, version-mismatch retry,
         pagination (MAX_PAGES), useServerTime, getSigner (factory precedence,
         L1/L2 auth unchanged)

Agent 19 — HTTP behavior + error handling
  OURS: src/lib.rs, src/clob/client.rs
  TS:   references/clob-client-v2/src/http-helpers/index.ts, src/errors.ts
  CHECK: retry (POST-only, 30ms, 1 retry), retry conditions (5xx, connect,
         timeout), default headers (User-Agent), gzip, non-2xx handling

Agent 20 — security review
  OURS: src/ (all files), Cargo.toml
  CHECK: HMAC base64 alphabets, SecretString on all credentials, salt masking,
         HTTPS enforcement, division-by-zero guards, strict UTF-8, address
         encoding, funder/sigtype validation, no unwrap() in production,
         no unsafe blocks, getSigner key leakage via Debug

## Report format

For each agent, use this format:

  AREA: [name]
  VERDICT: PASS / PASS WITH NOTES / FAIL
  FINDINGS: [bullet list of issues, if any, with file:line references]

After all 20 agents complete, compile a consolidated report with:

1. Overall V1 Convention Fidelity Score (out of 100)
2. Overall TS V2 Parity Score (out of 100)
3. Security Checklist (10 items, PASS/FAIL)
4. Build Verification (cargo check/test/clippy/doc — run these yourself)
5. Total test count
6. GO / NO-GO production recommendation
7. Ranked list of any remaining issues by severity (CRITICAL/HIGH/MEDIUM/LOW)

## Context from prior audits

Plans 0001-0004 document the full implementation and audit history. The most
recent audit (16 agents) scored V1 fidelity at 88/100 and TS V2 parity at
93/100 with a GO recommendation. All deductions were for intentional V2 changes
(dropped V1 order support, Decimal instead of IEEE 754, Result instead of
throwOnError), not bugs.

Known accepted items:
- WebSocket layer is minimal (feature-gated stub, not a full V1-style implementation)
- No tracing feature (V1 has it, V2 intentionally omits for now)
- No wallet derivation (derive_proxy_wallet/derive_safe_wallet are V1-only)
- Address encoding: auth headers use lowercase hex, order payloads use EIP-55 checksum

Your job is to independently verify these claims or find issues we missed.
```

---

## Notes

This prompt is designed to be copy-pasted into a fresh agent session with no prior context. The agent should:
1. Read the prompt
2. Spawn 20 subagents (10 convention + 10 correctness)
3. Compile results into a consolidated report
4. Give a GO/NO-GO recommendation

The prompt references exact file paths and TS reference paths so agents can do targeted reads without needing to explore the full tree first.
