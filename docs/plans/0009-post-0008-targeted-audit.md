# Plan 0009: Post-0008 Targeted Follow-Up Audit

**Status:** OPEN
**Created:** 2026-04-20
**Predecessor:** [0008-release-signoff-audit-prompt.md](0008-release-signoff-audit-prompt.md) (CLOSED)
**Target Commit:** `79332e5` `fix: remediate 0008 release signoff findings`
**Purpose:** Run a targeted independent follow-up audit against the post-0008 remediation set and decide whether the repository is now ready for a release-signoff verdict without another full 20-area repo sweep.

---

## 1. Why 0009 Exists

`0008` is the historical release-signoff audit record. It closed **NO-GO** and then received a documented post-audit remediation pass.

That remediation pass has now been implemented and pushed at `79332e5`, including:
- dynamic signer / `maker` resolution fixes
- pagination fixes with bounded iteration
- tracing-time secret-redaction hardening
- authenticated header-merge fixes with retry-path coverage
- market-order `GTC` / `GTD` rejection
- explicit market-order `price = 0` parity decision and regression coverage
- TS-number response-model reserialization fixes
- `Token.winner` serialization suppression with deserialize compatibility retained
- shared runtime caches across auth promotion
- additional unreleased parity tightening for `RewardsPercentages`, which now round-trips as a numeric JSON map rather than decimal strings

`0009` exists to verify those changes directly, not to relitigate the already-closed historical `0008` verdict.

---

## 2. Audit Goal

Determine whether current `HEAD` at `79332e5` is now:
- `GO`
- `GO WITH KNOWN LOW-RISK ITEMS`
- `NO-GO`

This should be a **targeted follow-up audit**, not another default full-repo 20-agent review, unless the targeted audit uncovers a new systemic issue.

**Primary question:** did the concrete `0008` blockers actually get fixed, and did the fixes introduce new regressions?

---

## 3. Audit Scope

### In Scope

1. `getSigner` / dynamic signer parity
2. auth-promotion cache sharing
3. pagination correctness and bounded iteration
4. tracing-mode secret redaction
5. authenticated request header merge behavior, including retry path
6. market-order API validation and `price = 0` semantics
7. response-model serialization parity for TS-number fields
8. `RewardsPercentages` numeric-map parity
9. smoke-check of previously fixed 0007 regressions to ensure they still hold
10. full build/test/clippy/doc verification matrix

### Out of Scope Unless New Evidence Appears

1. Another full 20-area repo sweep
2. Large architectural redesigns already accepted as intentional V1/V2 drift
3. New feature requests unrelated to the 0008 remediation set

If the audit finds a new **HIGH** issue outside this scope, expand only as needed to explain the risk.

---

## 4. Required Verification Matrix

Run these commands directly against current `HEAD`:

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

**Current expected baseline at `79332e5`:**
- default test count: **126**
- `ws` test count: **131**
- `rg -n "https://clob\\.polymarket\\.com" tests`: **no matches**

A mismatch does not automatically fail the audit, but it must be explained.

---

## 5. Suggested Audit Topology

Use **6 to 8 focused agents** if the runtime permits. Do not spend 20 agents on a localized remediation follow-up unless the findings justify it.

### Area 1 — Dynamic Signer / Maker / Auth Promotion Caches

**OURS**
- `src/clob/order_builder.rs`
- `src/clob/client.rs`
- `tests/get_signer.rs`
- `tests/client.rs`

**TS / V1 REFERENCES**
- `references/clob-client-v2/src/order-builder/orderBuilder.ts`
- `references/clob-client-v2/src/order-builder/helpers/createOrder.ts`
- `references/clob-client-v2/src/order-builder/helpers/createMarketOrder.ts`
- `references/rs-clob-client/src/clob/client.rs`

**CHECK**
- `maker` is derived from `(explicit funder) || (resolved signer)` at final build time
- builder-level and config-level `get_signer` flows both serialize the correct `maker`
- auth promotion shares runtime cache state rather than copying into detached maps

### Area 2 — Market-Order Validation / `price = 0`

**OURS**
- `src/clob/order_builder.rs`
- `src/clob/client.rs`
- `tests/order.rs`

**TS REFERENCES**
- `references/clob-client-v2/src/client.ts`
- `references/clob-client-v2/src/order-builder/helpers/getMarketOrderRawAmounts.ts`
- `references/clob-client-v2/src/utilities.ts`

**CHECK**
- only `FOK | FAK` are accepted for market orders
- unsupported `GTC` / `GTD` fail at the public Rust entrypoints
- explicit `price = 0` behavior is internally consistent and documented
- no regression to price-bound validation

### Area 3 — Pagination / Header Merge / Retry Path

**OURS**
- `src/clob/client.rs`
- `src/lib.rs`
- `tests/client.rs`

**TS / V1 REFERENCES**
- `references/clob-client-v2/src/client.ts`
- `references/clob-client-v2/src/http-helpers/index.ts`
- `references/rs-clob-client/src/clob/client.rs`

**CHECK**
- pagination advances by cursor rather than stopping on empty non-terminal pages
- a hard upper bound exists on pagination loops
- auth headers merge onto existing request headers rather than replacing them
- retry preserves those merged headers

### Area 4 — Tracing / Secret Redaction

**OURS**
- `src/serde_helpers.rs`
- `src/clob/client.rs`

**CHECK**
- tracing-enabled deserialization does not emit raw secret-bearing values
- redaction covers normal logging, error paths, and unknown-field warnings
- secret-bearing response `Debug` output remains redacted

### Area 5 — Response-Model Serialization Parity

**OURS**
- `src/clob/types/market.rs`
- `src/clob/types/response.rs`
- `tests/serde.rs`

**TS REFERENCES**
- `references/clob-client-v2/src/types/clob.ts`

**CHECK**
- TS-number fields serialize back as JSON numbers where required
- `Token.winner` stays deserialize-compatible but is omitted from TS-shaped output
- `Page<T>.limit/count` remain `u64`
- `RewardsPercentages` now round-trips as a numeric map, not decimal strings

### Area 6 — Prior Regression Smoke Check

**OURS**
- `src/clob/client.rs`
- `src/clob/types/market.rs`
- `tests/client.rs`
- `tests/serde.rs`
- `tests/order.rs`

**CHECK**
- `builder_fees()`, `current_rewards()`, `raw_rewards_for_market()` remain public
- version-mismatch retry still covers success-shaped and non-2xx mismatch payloads
- REST insecure host validation still only permits `http://` via explicit opt-in
- shorthand `MarketDetails.fd.r` still serializes as a number

---

## 6. Decision Rules

- If any unresolved **HIGH** issue remains, verdict is `NO-GO`.
- If only **MEDIUM** / **LOW** issues remain, verdict may be:
  - `GO WITH KNOWN LOW-RISK ITEMS`, or
  - `NO-GO` if the remaining issues cluster around one risky subsystem.
- If the targeted remediation set is verified and no new material issues appear, verdict is `GO`.

**Do not recommend another full 20-agent audit unless:**
- a new `HIGH` issue is found,
- multiple medium issues suggest broader systemic drift,
- or the build/test/clippy/doc matrix is no longer green.

---

## 7. Required Report Format

For each area, report:

- `AREA:` [name]
- `VERDICT:` `PASS` / `PASS WITH NOTES` / `FAIL`
- `FINDINGS:`
  - concrete file:line references for any issue

Then provide:

1. Build verification results for the full matrix
2. Default and `ws` test counts
3. Security checklist delta from `0008`
4. Final verdict: `GO` / `GO WITH KNOWN LOW-RISK ITEMS` / `NO-GO`
5. Explicit statement whether `0008`’s remediated blockers are now actually closed
6. Explicit statement whether another **full 20-agent** audit is necessary

---

## 8. Auditor Guidance

This follow-up should be adversarial but efficient.

- Treat `0008` as a historical record, not as truth.
- Verify current `HEAD` directly.
- Focus on whether the remediation at `79332e5` is correct and complete.
- Do not inflate the audit back into a full-repo re-review unless the new evidence requires it.
