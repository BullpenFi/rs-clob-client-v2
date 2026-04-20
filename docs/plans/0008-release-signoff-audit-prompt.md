# Plan 0008: Release Signoff Audit Prompt — Closure & Findings

**Status:** CLOSED
**Created:** 2026-04-20
**Closed:** 2026-04-20
**Predecessors:** [0005-audit-prompt.md](0005-audit-prompt.md) (CLOSED), [0006-independent-audit-findings.md](0006-independent-audit-findings.md), [0007-independent-reaudit-prompt.md](0007-independent-reaudit-prompt.md) (CLOSED)
**Purpose:** Preserve the final release-signoff audit prompt and record the adjudicated outcome of the full-scope, two-pass, 20-area independent audit run against current `rs-clob-client-v2` `HEAD`.

---

## 1. Release-Signoff Outcome

The release-signoff audit was executed against current `HEAD` using the same two-pass, 20-area structure defined in this plan. The runtime still capped concurrent subagents below 20, so the audit ran in parallel sub-batches, but all 20 requested scopes completed.

**Final scores:**
- Overall V1 convention fidelity: **90/100**
- Overall TS V2 parity: **88/100**
- Final release verdict: **NO-GO**

**Top-level adjudication:**
- The repository's self-reported **green build/test/clippy/doc matrix** claim is supported.
- The repository's self-reported **GO candidate** claim is **not** supported.
- Another full 20-agent audit is **not currently necessary**. The remaining work is localized enough for a targeted follow-up audit after fixes.

**Reason for NO-GO:**
- A **HIGH** issue remains in the core order-construction path: dynamic `getSigner` / signer-factory flows can still build orders with the wrong `maker` address when no explicit funder is set.
- Several medium-risk issues remain around pagination behavior, authenticated request header preservation, tracing-time secret exposure, and market-order API validation.

---

## 2. Priority Regression Adjudication

The prompt required the audit to verify five specific regressions from `0007` before broad review. The release-signoff outcome was:

1. Market-order price bounds
   - **Supported.** Explicit market-order prices now enforce the TS-valid inclusive range `[tick_size, 1 - tick_size]`, and [tests/order.rs](../../tests/order.rs) covers out-of-range rejection.
2. Version-mismatch retry logic
   - **Supported.** Rust now detects `order_version_mismatch` on both success-shaped and status-shaped responses, refreshes `/version`, and only retries when the refreshed version changed.
3. Public endpoint exposure
   - **Supported.** `builder_fees()`, `current_rewards()`, and `raw_rewards_for_market()` are now exposed on the public unauthenticated client surface and covered in [tests/client.rs](../../tests/client.rs).
4. Shorthand market serialization
   - **Supported.** `MarketDetails.fd.r` now serializes as a numeric shorthand field, and [tests/serde.rs](../../tests/serde.rs) asserts the numeric output.
5. REST host insecure escape hatch
   - **Supported.** Main REST host validation now mirrors the narrower builder/websocket scheme policy: `https://` by default, `http://` only with explicit insecure opt-in, and other schemes are rejected.

These five prior blockers are no longer the reason for the signoff failure.

---

## 3. Verification Captured During Release-Signoff Audit

**Verification baseline captured directly during the audit:**
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
- `rg -n "https://clob\\.polymarket\\.com" tests` — **no matches**

---

## 4. Security Checklist

1. HMAC base64 decode/encode alphabets — **PASS**
2. Secret-bearing persistent credentials and builder tokens use `SecretString` — **PASS**
3. Salt generation stays within the JS-safe 53-bit range — **PASS**
4. Main REST host validation defaults to `https://` and only permits explicit `http://` opt-in — **PASS**
5. Remote builder signing defaults to `https://` and only permits explicit `http://` opt-in — **PASS**
6. Websocket validation defaults to `wss://` and only permits explicit `ws://` opt-in — **PASS**
7. Version-mismatch retry covers both success-shaped and non-2xx mismatch payloads — **PASS**
8. Market-order explicit user prices enforce TS-valid bounds — **PASS**
9. Secret-bearing debug/tracing output is consistently redacted — **FAIL**
10. No `unsafe` blocks were found under `src/` — **PASS**

---

## 5. Agent Adjudication Matrix

This section records all 20 scoped review areas and the final adjudicated result from the release-signoff audit.

1. `lib.rs + module structure` — **PASS WITH NOTES**
   - `src/lib.rs:29-44` and `src/lib.rs:64-118` keep a lighter tracing/diagnostic shape than V1. The drift is real but was not treated as a release blocker.
2. `error.rs` — **PASS**
   - No material issues found.
3. `auth.rs` — **PASS WITH NOTES**
   - `src/auth.rs:400-463` is stricter than V1 on request-body handling. That is acceptable for the current JSON/UTF-8 request surface, but it is not a byte-for-byte V1 clone.
4. `client.rs architecture` — **FAIL**
   - `src/clob/client.rs:469-475` still truncates pagination on empty non-terminal pages.
   - `src/clob/client.rs:155-217` still clones cache state during auth promotion rather than preserving a single inner like V1.
5. `order_builder.rs` — **FAIL**
   - `src/clob/order_builder.rs:123-125`, `300-304`, and `420-422` still snapshot `maker` before dynamic signer resolution.
   - `src/clob/order_builder.rs:365` still accepts unsupported market-order `OrderType` values.
6. `serde_helpers + config + shared types` — **PASS WITH NOTES**
   - `src/serde_helpers.rs:15-60` correctly restores tracing-gated warning-preserving deserialization, but the same tracing path can now expose sensitive payloads if enabled.
7. `request/response types` — **PASS WITH NOTES**
   - `src/clob/types/response.rs:75-80` narrows `Page<T>.limit/count` to `u32` instead of the broader V1 `u64`.
8. `websocket infrastructure` — **PASS**
   - No material issues found.
9. `Cargo.toml + dependencies` — **PASS WITH NOTES**
   - `Cargo.toml:61-68` intentionally relaxes two allow-attribute clippy lints versus V1, and the crate does not currently declare an explicit `rust-version`.
10. `tests + examples + README` — **PASS WITH NOTES**
   - The suite is hermetic and the README positioning is accurate, but websocket coverage remains validation-only and example coverage remains narrower than V1.
11. `EIP-712 signing` — **PASS**
   - No parity issues found.
12. `HMAC + L1/L2 authentication` — **PASS WITH NOTES**
   - Lowercase auth-header address formatting remains a known intentional difference from TS mixed-case preservation and was not treated as a blocker.
13. `order amount math` — **PASS WITH NOTES**
   - Market-order bounds are now fixed, but explicit `price = 0` still diverges from TS fallback semantics.
14. `market price calculation` — **PASS**
   - No parity issues found.
15. `endpoint paths + HTTP methods` — **PASS**
   - No parity issues found.
16. `POST payloads` — **PASS**
   - No parity issues found.
17. `type serialization + constants` — **FAIL**
   - `src/clob/types/market.rs:181-236` and `src/clob/types/response.rs:103-114` still serialize several TS-number model fields through `Decimal`, and `Token` still serializes an extra `winner` field not present in the TS type.
18. `client method logic + getSigner` — **FAIL**
   - Dynamic signer precedence is still incomplete because signer resolution does not update maker fallback semantics.
19. `HTTP behavior + error handling` — **FAIL**
   - `src/lib.rs:80-81` still replaces, rather than merges, per-request auth headers, dropping client default headers on authenticated traffic.
20. `security review` — **FAIL**
   - `src/serde_helpers.rs:18-21`, `40-45`, and `55-58` can log secret-bearing JSON under the `tracing` feature.
   - `src/clob/client.rs:67-73` derives `Debug` for a secret-bearing API-key response type without redaction.

---

## 6. Confirmed Remaining Issues

### HIGH

#### H1: Dynamic signer flows can still build orders with the wrong maker address

**Files:** `src/clob/order_builder.rs:123-125`, `src/clob/order_builder.rs:300-304`, `src/clob/order_builder.rs:420-422`

`OrderBuilder::new()` snapshots `maker` from the static client signer once, before any dynamic signer is resolved. Later, `build()` and `build_with_signer()` correctly resolve the active signer address, but both limit and market orders still serialize the stale `self.maker` into the order payload. In the TS client, maker fallback follows the resolved signer address when no explicit funder is provided.

**Why it matters:** This is a real order-construction bug in a core trading path. For configs that use `getSigner` or a signer factory without an explicit funder, Rust can sign an order with one signer while claiming funds from a different maker address.

**Required fix:**
- Stop storing maker as a fixed field derived at builder construction time.
- Derive maker from `(explicit funder) || (resolved signer address)` at final build time.
- Add regression coverage for both limit and market orders using `getSigner` / signer-factory flows without explicit funders.

### MEDIUM

#### M1: Pagination still truncates on the first empty page

**Files:** `src/clob/client.rs:469-475`, `tests/client.rs:774-810`

`collect_pages()` still breaks immediately when a page returns an empty `data` array, even if `next_cursor` is not the terminal cursor. V1 and TS advance strictly by cursor and only stop at the terminal cursor.

**Why it matters:** Sparse paginated endpoints can silently lose data after a transient empty page.

**Required fix:**
- Continue iterating until the terminal cursor regardless of page emptiness.
- Replace the current `pagination_stops_on_empty_page` expectation with cursor-faithful regression coverage.

#### M2: Tracing-enabled builds can leak secret-bearing payloads

**Files:** `src/serde_helpers.rs:18-21`, `src/serde_helpers.rs:40-45`, `src/serde_helpers.rs:55-58`, `src/clob/client.rs:67-73`

Under `feature = "tracing"`, `deserialize_with_warnings()` logs full JSON values during normal deserialization, error-path inspection, and unknown-field reporting. That can include API-key `secret` and `passphrase` values. Separately, `CreateApiKeyResponse` still derives `Debug` without redaction.

**Why it matters:** This is a real credential-leak risk in tracing-enabled builds.

**Required fix:**
- Redact or suppress sensitive fields before logging JSON values.
- Replace raw `Debug` derivation on secret-bearing response types with a manual redacted implementation, or remove `Debug` if it is not needed.

#### M3: Authenticated requests still drop default transport headers

**Files:** `src/lib.rs:80-81`, `src/clob/client.rs:928-935`, `src/clob/client.rs:1039-1064`

Per-request auth headers still replace the request header map instead of merging onto the client defaults. Authenticated requests therefore drop `User-Agent`, `Accept`, `Connection`, and `Content-Type` even though those defaults are configured on client construction.

**Why it matters:** This is a transport-behavior mismatch from the TS helper and can produce inconsistent behavior between public and authenticated requests.

**Required fix:**
- Merge auth headers into the existing header map instead of replacing it wholesale.
- Add regression coverage asserting that authenticated requests preserve default headers.

#### M4: Market-order APIs still accept unsupported `GTC`/`GTD` order types

**Files:** `src/clob/order_builder.rs:44-52`, `src/clob/order_builder.rs:365`, `src/clob/client.rs:1395-1397`, `src/clob/client.rs:1450-1464`

Rust `UserMarketOrder` still stores `order_type: Option<OrderType>`, and both `create_market_order()` and `create_and_post_market_order()` can feed arbitrary `OrderType` values into market-order construction. TS restricts market orders to `FOK | FAK`.

**Why it matters:** Invalid market-order execution modes can slip through Rust validation instead of being rejected at the API boundary.

**Required fix:**
- Narrow the public market-order type surface to `Fok | Fak`.
- Reject unsupported order types explicitly if type narrowing is not practical.
- Add regression coverage for rejected `GTC` and `GTD` market orders.

### LOW

#### L1: Explicit market-order `price = 0` still diverges from TS semantics

**File:** `src/clob/order_builder.rs:375-385`

Rust treats `Some(0)` as an explicit price and rejects it via `validate_price()`. TS treats falsy `price` as absent and computes market price before validation.

**Why it matters:** This is a narrow edge-case parity difference on explicit market-order input handling.

**Required fix:**
- Decide whether Rust should preserve explicit-zero rejection or mirror TS falsy fallback exactly.
- Add a regression test documenting whichever behavior is chosen.

#### L2: Some public model types still re-serialize TS-number fields as decimal strings

**Files:** `src/clob/types/market.rs:181-236`, `src/clob/types/response.rs:103-114`

Several public response-model fields are stored as `Decimal`, so re-serialization emits JSON strings where the TS reference type uses `number`. `Token` also still serializes an extra `winner` field not present in the TS type.

**Why it matters:** This is primarily a public model re-serialization drift rather than a request-path bug, but it still weakens type-level parity.

**Required fix:**
- Add numeric serializers or wrappers where wire-level numeric parity is required.
- Decide whether `winner` is an intentional Rust extension; if so, document it clearly or suppress serialization for TS-shaped output models.

---

## 7. Rejected Or Downgraded Candidate Findings

During the 20-scope review, several exploratory findings were reviewed and intentionally **not** carried forward as release blockers:

1. `create_or_derive_api_key()` fallback breadth
   - The Rust fallback-on-create-failure path was reviewed against the TS default `post()` helper, which returns error objects instead of throwing when `throwOnError` is disabled. This was not retained as a new blocker.
2. Lowercase `POLY_ADDRESS` header formatting
   - Rust still normalizes the auth address to lowercase hex. That remains a known intentional difference and was not escalated in this signoff.
3. Narrower insecure transport policy not restricted to loopback only
   - REST, builder, and websocket surfaces now all require explicit insecure opt-in and constrain accepted insecure schemes correctly. The audit did not escalate the lack of loopback-only restriction as a separate blocker.
4. V1 tracing-shape and observability drift
   - The lighter tracing shape in `lib.rs` remains a conventions note, not a release blocker by itself.

---

## 8. Post-Audit Remediation Update

After this audit closed **NO-GO**, the adjudicated findings in Sections 5 and 6 were remediated in the working tree and re-verified locally. This addendum preserves that follow-up without rewriting the original release-signoff verdict above.

**Remediations applied after the audit closed:**
- Fixed dynamic signer / `getSigner` maker resolution so order `maker` now follows `(explicit funder) || (resolved signer)` at final build time.
- Fixed pagination to advance strictly by cursor while keeping a hard upper bound on page count.
- Fixed tracing-time secret exposure by removing raw serde error-text logging, keeping redacted value logging, and redacting secret-bearing API-key response debug output.
- Fixed authenticated request header handling so per-request auth headers merge onto existing request headers instead of replacing them.
- Fixed market-order validation so unsupported `GTC` / `GTD` values are rejected at the builder and public client entrypoints.
- Resolved the explicit market-order `price = 0` parity decision by documenting and testing the TS-style fallback to computed market price.
- Fixed the adjudicated TS-number re-serialization gaps in the audited model slice and suppressed `Token.winner` on TS-shaped output serialization while retaining deserialize compatibility.
- Tightened one additional unreleased parity gap beyond the ranked 0008 findings by converting `RewardsPercentages` from a raw alias into a serde-aware wrapper so it now round-trips as a numeric JSON map instead of decimal strings.
- Fixed auth promotion cache sharing so authenticated clients now preserve shared runtime caches instead of copying cache state into a fresh inner.

**Post-audit verification rerun after remediation:**
- `cargo check` — PASS
- `cargo check --no-default-features` — PASS
- `cargo check --no-default-features --features ws` — PASS
- `cargo check --all-features` — PASS
- `cargo test` — PASS
- `cargo test --features ws` — PASS
- `cargo clippy --all-targets --all-features -- -D warnings` — PASS
- `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` — PASS

**Record-keeping note:**
- This section records that the concrete issues from the final 0008 audit were addressed and the full verification matrix was rerun successfully.
- It does **not** retroactively change the historical `NO-GO` outcome captured in Section 1. A new targeted follow-up audit should make the next GO / NO-GO determination.

---

## 9. Archived Prompt Claim Snapshot

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

## 10. Archived Release Signoff Rules

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

## 11. Archived 0007 Closure Context

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

## 12. Original Release-Signoff Audit Prompt

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
