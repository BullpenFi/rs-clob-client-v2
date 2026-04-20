# Plan 0008: Release Signoff Audit Prompt

**Status:** Ready
**Created:** 2026-04-20
**Predecessors:** [0005-audit-prompt.md](0005-audit-prompt.md) (CLOSED), [0006-independent-audit-findings.md](0006-independent-audit-findings.md), [0007-independent-reaudit-prompt.md](0007-independent-reaudit-prompt.md) (CLOSED)
**Purpose:** Archive the final full-scope, two-round, 20-agent independent audit prompt for release signoff of the current `rs-clob-client-v2` repository state.

---

## Current Repository Claim

The repository owner now claims the latest remediation pass resolved the remaining material blockers from `0007`, and that the crate is a legitimate **GO candidate pending one final release-signoff audit**.

**Current audit target:** `HEAD` at commit `ddfc818`

**Most recent remediation commit:** `2ef0e6b` `fix: resolve remaining parity audit findings`

**Latest claimed verification baseline at HEAD:**
- `cargo check` — PASS
- `cargo check --no-default-features` — PASS
- `cargo check --no-default-features --features ws` — PASS
- `cargo check --all-features` — PASS
- `cargo test` — PASS
- `cargo test --features ws` — PASS
- `cargo clippy --all-targets --all-features -- -D warnings` — PASS
- `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` — PASS
- `cargo test -- --list` — **112 tests**
- `cargo test --features ws -- --list` — **117 tests**

**Claims this audit must verify, not assume:**
1. Market-order validation now matches TS V2 bounds for explicit user-supplied prices.
2. `create_and_post_*` retry behavior now matches TS V2 for non-2xx `order_version_mismatch` payloads and unchanged-version refreshes.
3. `builder_fees()`, `current_rewards()`, and `raw_rewards_for_market()` are now correctly exposed on the public Rust client surface.
4. `MarketDetails.fd.r` now serializes as a numeric shorthand field, not a string.
5. Main REST `allow_insecure` now only permits `http://` as the local-dev escape hatch and rejects other non-HTTPS schemes.
6. The full build/test/clippy/doc matrix is still green after these behavior changes.
7. No new regressions were introduced elsewhere in the repo while fixing the `0007` findings.

This prompt is intentionally adversarial. The audit should try to disprove the GO candidacy if the code does not actually support it.

---

## Release Signoff Rules

This is the final planned full-repo, 20-agent audit unless it discovers new substantial issues.

**Required decision rule:**
- If any `CRITICAL` or `HIGH` issue remains, the verdict is **NO-GO**.
- If only `MEDIUM` and `LOW` issues remain, the verdict may be either:
  - **GO WITH KNOWN LOW-RISK ITEMS**, or
  - **NO-GO**, if the medium findings are clustered around a single risky subsystem.
- A clean release-signoff verdict is **GO**.

**Do not recommend another full 20-agent audit unless one of these is true:**
- a new `CRITICAL` or `HIGH` issue is found,
- multiple medium findings suggest a broader systemic gap,
- or the verification matrix is no longer green.

---

## 0007 Closure Context

The immediately preceding full-repo re-audit (`0007`) closed with:
- Overall V1 convention fidelity: **92/100**
- Overall TS V2 parity: **80/100**
- Production recommendation: **NO-GO**

`0007` confirmed five remaining issues:
1. Market-order user price bounds still did not match TS V2.
2. Version-mismatch retry logic still did not match TS V2 on non-2xx responses and unchanged version refreshes.
3. Three TS-public GET endpoints were still hidden behind the authenticated Rust client surface.
4. `MarketDetails.fd.r` still serialized as a string.
5. Main REST `allow_insecure` still allowed arbitrary non-HTTPS schemes.

The owner claims those five issues were remediated in commit `2ef0e6b` and are the main regression targets for this signoff audit.

---

## Release-Signoff Audit Prompt

Copy everything below this line and give it to a new agent session.

---

```text
# Task: Final Release-Signoff Independent Audit of rs-clob-client-v2

You are auditing a Rust crate that reimplements Polymarket's TypeScript CLOB client V2
in Rust, following the architectural patterns of Polymarket's official Rust V1 SDK.

## Repository

/Users/hongjunwu/Documents/Git/rs-clob-client-v2/

## Layout

src/                          # The implementation under audit
tests/                        # Integration tests
examples/                     # Example programs
references/rs-clob-client/    # Polymarket's official Rust V1 SDK (STYLE GUIDE)
references/clob-client-v2/    # Polymarket's TypeScript V2 SDK (FEATURE SPEC)
docs/plans/                   # Implementation and audit history (0001-0008)

## What this crate does

It is a Rust trading SDK for Polymarket's CLOB (Central Limit Order Book) V2 API.
It handles EIP-712 order signing, HMAC-SHA256 API authentication, order building
with decimal math, REST endpoint coverage for the Polymarket trading API, and a
feature-gated websocket surface.

## Your task

Run one final full-scope independent audit using the same two-pass, 20-area structure
used in the earlier audits.

Spawn 20 subagents if the runtime permits it. If not, preserve the same 20 scoped
review areas and execute them in parallel sub-batches until all 20 complete.

This is a release-signoff audit. Do not treat it as a casual code review. The point
is to decide whether current HEAD deserves:
- GO
- GO WITH KNOWN LOW-RISK ITEMS
- NO-GO

You must verify current HEAD directly. Do not trust prior plan text, prior findings,
or prior remediation claims unless the current source and verification commands support them.

## Priority Regression Focus

Before broad review, explicitly verify these five areas from the prior NO-GO audit:

1. Market-order price bounds
   OURS: src/clob/order_builder.rs, tests/order.rs
   TS:   references/clob-client-v2/src/client.ts, src/utilities.ts,
         src/order-builder/helpers/getMarketOrderRawAmounts.ts
   CLAIM: explicit market-order prices now obey TS-valid inclusive range
          `[tick_size, 1 - tick_size]`

2. Version-mismatch retry logic
   OURS: src/clob/client.rs, src/lib.rs, src/error.rs
   TS:   references/clob-client-v2/src/client.ts,
         src/http-helpers/index.ts, src/constants.ts
   CLAIM: Rust now detects `order_version_mismatch` on both success-shaped and
          non-2xx status-shaped responses, refreshes `/version`, and retries only
          when the refreshed version changed

3. Public endpoint exposure
   OURS: src/clob/client.rs, tests/client.rs
   TS:   references/clob-client-v2/src/client.ts, src/endpoints.ts
   CLAIM: `builder_fees()`, `current_rewards()`, and `raw_rewards_for_market()` now
          live on the public Rust client surface like TS public reads

4. Shorthand market serialization
   OURS: src/clob/types/market.rs, tests/serde.rs
   TS:   references/clob-client-v2/src/types/clob.ts
   CLAIM: `MarketDetails.fd.r` now serializes as a JSON number, not a string

5. REST host insecure escape hatch
   OURS: src/clob/client.rs, tests/client.rs
   REFS: src/auth.rs, src/ws/config.rs, src/ws/connection.rs
   CLAIM: main REST host validation now mirrors the narrower builder/ws scheme
          policy: `https://` by default, `http://` only with explicit insecure opt-in,
          all other schemes rejected

If any of those five regressions are still wrong, the result is automatically NO-GO.

## Pass 1: Convention Audit (10 agents) — compare against Rust V1 SDK

Agent 1 — lib.rs + module structure
  OURS: src/lib.rs, src/clob/mod.rs
  V1:   references/rs-clob-client/src/lib.rs, src/clob/mod.rs
  CHECK: feature gates, re-exports, request() handler, ToQueryParams,
         doc comments, tracing feature shape, lint allowances

Agent 2 — error.rs
  OURS: src/error.rs
  V1:   references/rs-clob-client/src/error.rs
  CHECK: Kind variants, Error struct shape, backtrace, downcast_ref,
         Display/From impls, status payload preservation

Agent 3 — auth.rs
  OURS: src/auth.rs
  V1:   references/rs-clob-client/src/auth.rs
  CHECK: state/sealed traits, Credentials secrecy, L1 EIP-712 auth,
         L2 HMAC, builder auth, remote signer validation, header naming

Agent 4 — client.rs architecture
  OURS: src/clob/client.rs
  V1:   references/rs-clob-client/src/clob/client.rs
  CHECK: Client<State>, Arc<Inner>, Config, auth promotion, caches,
         default headers, pagination, retry boundaries, state transitions

Agent 5 — order_builder.rs
  OURS: src/clob/order_builder.rs
  V1:   references/rs-clob-client/src/clob/order_builder.rs
  CHECK: PhantomData type-state, price/size validation, salt masking,
         amount math, getSigner precedence, builder ergonomics

Agent 6 — serde_helpers + config + shared types
  OURS: src/serde_helpers.rs, src/config.rs, src/types.rs
  V1:   references/rs-clob-client/src/serde_helpers.rs, src/types.rs
  CHECK: tracing-gated deserializer path, helper shapes, phf config,
         Decimal/Address re-exports

Agent 7 — request/response types
  OURS: src/clob/types/request.rs, response.rs, book.rs, trade.rs, market.rs, builder.rs
  V1:   references/rs-clob-client/src/clob/types/request.rs, response.rs
  CHECK: bon builders, serde renames, Page<T>, string-vs-numeric conventions,
         structured response wrappers

Agent 8 — websocket infrastructure
  OURS: src/ws/, src/clob/ws/
  V1:   references/rs-clob-client/src/ws/, src/clob/ws/
  CHECK: feature gates, connection guardrails, allow_insecure semantics,
         intentional surface reduction vs missing required behavior

Agent 9 — Cargo.toml + dependencies
  OURS: Cargo.toml
  V1:   references/rs-clob-client/Cargo.toml
  CHECK: dependency choices, feature flags, lint config, reqwest features,
         tracing/serde_ignored shape, gzip/rustls wiring

Agent 10 — tests + examples + README
  OURS: tests/, examples/, README.md
  V1:   references/rs-clob-client/tests/, examples/
  CHECK: hermeticity, httpmock usage, ws coverage, example completeness,
         README accuracy about unofficial Bullpen.fi ownership and official SDK guidance

## Pass 2: Correctness Audit (10 agents) — compare against TypeScript V2 SDK

Agent 11 — EIP-712 signing
  OURS: src/clob/types/order.rs, src/config.rs
  TS:   references/clob-client-v2/src/order-utils/model/ctfExchangeV2TypedData.ts,
        src/order-utils/exchangeOrderBuilderV2.ts, src/order-utils/model/orderDataV2.ts,
        src/config.ts
  CHECK: typed data fields, domain version, contract addresses, signature type encoding,
         hash/signature parity

Agent 12 — HMAC + L1/L2 authentication
  OURS: src/auth.rs
  TS:   references/clob-client-v2/src/signing/hmac.ts, src/headers/index.ts,
        src/signing/eip712.ts, src/signing/constants.ts
  CHECK: HMAC message shape, base64 alphabets, auth typed data,
         header names, address format

Agent 13 — order amount math
  OURS: src/clob/order_builder.rs
  TS:   references/clob-client-v2/src/order-builder/helpers/getOrderRawAmounts.ts,
        getMarketOrderRawAmounts.ts, roundingConfig.ts, utilities.ts, client.ts
  CHECK: RoundConfig, limit math, market math, rounding, fee adjustment,
         explicit market-price bounds

Agent 14 — market price calculation
  OURS: src/clob/client.rs
  TS:   references/clob-client-v2/src/order-builder/helpers/calculateBuyMarketPrice.ts,
        calculateSellMarketPrice.ts
  CHECK: traversal order, accumulation semantics, FOK guard, FAK fallback

Agent 15 — endpoint paths + HTTP methods
  OURS: src/clob/client.rs
  TS:   references/clob-client-v2/src/endpoints.ts, src/client.ts
  CHECK: endpoint paths, HTTP methods, auth levels, public vs authenticated surface,
         query-vs-body placement, builder/rewards/scoring coverage

Agent 16 — POST payloads
  OURS: src/clob/client.rs, src/clob/types/*
  TS:   references/clob-client-v2/src/types/ordersV2.ts, src/client.ts
  CHECK: field renames, side encoding, signatureType encoding, salt shape,
         taker omission, expiration placement

Agent 17 — type serialization + constants
  OURS: src/clob/types/, src/config.rs
  TS:   references/clob-client-v2/src/types/clob.ts, src/config.ts, src/constants.ts
  CHECK: camelCase renames, shorthand market fields, `fd.r` numeric output,
         enum values, tick-size representation, contract config

Agent 18 — client method logic + getSigner
  OURS: src/clob/client.rs, src/clob/order_builder.rs
  TS:   references/clob-client-v2/src/client.ts, src/order-builder/orderBuilder.ts
  CHECK: createOrDerive fallback semantics, version caching, mismatch retry,
         useServerTime, getSigner scope and precedence

Agent 19 — HTTP behavior + error handling
  OURS: src/lib.rs, src/clob/client.rs, src/error.rs
  TS:   references/clob-client-v2/src/http-helpers/index.ts, src/errors.ts
  CHECK: retry boundaries, retry conditions, default headers, gzip behavior,
         structured non-2xx handling, preserved payloads, message derivation

Agent 20 — security review
  OURS: src/, Cargo.toml
  CHECK: HMAC alphabet correctness, SecretString usage, 53-bit salt range,
         HTTPS/WSS enforcement, insecure escape hatch scope, UTF-8 strictness,
         debug redaction, unsafe usage

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
- `rg -n "https://clob\\.polymarket\\.com" tests`

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
6. Final release verdict:
   - GO
   - GO WITH KNOWN LOW-RISK ITEMS
   - NO-GO
7. Ranked list of remaining issues by severity (CRITICAL/HIGH/MEDIUM/LOW)
8. Explicit statement whether the repo's current GO-candidate claim is supported
9. Explicit statement whether another full 20-agent audit is necessary or not

## Context from prior audits

Plans 0001-0007 document the implementation and earlier audit history.

The prior full release-blocking re-audit (0007) produced:
- Overall V1 convention fidelity: 92/100
- Overall TS V2 parity: 80/100
- Production recommendation: NO-GO

The owner claims the five confirmed `0007` blockers were fixed in commit `2ef0e6b`.
Your job is to independently verify or reject that claim at current HEAD.
```

---

## Notes

This prompt is meant to be copied into a fresh agent session with no prior context. The auditor should:
1. Read the prompt
2. Run the required verification commands directly
3. Execute all 20 review scopes across the two passes
4. Produce a final release-signoff verdict with a clear recommendation on whether another full audit is necessary

This plan is intentionally framed as a terminal signoff audit, not another indefinite audit loop.
