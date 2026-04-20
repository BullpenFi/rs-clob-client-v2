# Plan 0010: Final Codex Full-Repo Polish Audit — Closure & Findings

**Status:** CLOSED
**Created:** 2026-04-20
**Closed:** 2026-04-20
**Predecessors:** [0008-release-signoff-audit-prompt.md](0008-release-signoff-audit-prompt.md) (CLOSED), [0009-post-0008-targeted-audit.md](0009-post-0008-targeted-audit.md) (CLOSED)
**Audit Target Commit:** `7ed79b4` `docs: add plan 0009 targeted audit`
**Post-Audit Cleanup Commit:** `00a68ff` `fix: close final low-risk audit gaps`
**Purpose:** Preserve the final broad Codex full-repo polish/signoff audit prompt and record the adjudicated outcome of the last planned broad review pass across the repository.

---

## 1. Final Broad Audit Outcome

The final broad Codex audit ran across the full repository at current `HEAD`, using the full-repo scope described in this plan rather than the narrower targeted follow-up from `0009`.

**Reviewer-scored outcome:**
- Overall V1 convention fidelity: **95/100**
- Overall TS V2 parity: **98/100**
- Initial final verdict: **GO WITH KNOWN LOW-RISK ITEMS**

**Top-level adjudication:**
- No material runtime, parity, or security issue remained.
- Current `HEAD` held up under the final broad pass.
- The only remaining items were two low-risk polish issues:
  - `Cargo.toml` still pointed `repository` at Polymarket instead of BullpenFi
  - direct regression coverage for the success-shaped `order_version_mismatch` retry path was weaker than the status-shaped branches
- Another broad repo audit was **not necessary**. The reviewer explicitly recommended direct cleanup of those two items rather than another audit loop.

---

## 2. Area Adjudication Matrix

1. `Crate structure, modules, features` — **PASS WITH NOTES**
   - Low-risk note: `Cargo.toml:9` repository metadata pointed at the wrong GitHub repository.
2. `Error system and request core` — **PASS**
   - No findings.
3. `Auth, signing infrastructure, secrecy` — **PASS**
   - No findings.
4. `Client architecture and caches` — **PASS WITH NOTES**
   - Note: auth promotion still builds a fresh `ClientInner` while sharing runtime maps by `Arc`. No functional regression was found.
5. `Order builder and math pipeline` — **PASS**
   - No findings.
6. `Tests, examples, README` — **PASS WITH NOTES**
   - Low-risk note: the success-shaped `order_version_mismatch` retry branch was correct by inspection, but direct regression coverage was stronger on the status-shaped and unchanged-version branches.
7. `EIP-712 order model and signing parity` — **PASS**
   - No findings.
8. `L1/L2 auth header parity` — **PASS**
   - No findings.
9. `Endpoint surface and auth placement` — **PASS**
   - No findings.
10. `Market-order behavior and execution helpers` — **PASS**
   - No findings.
11. `Response/request model parity` — **PASS**
   - No findings.
12. `Version mismatch, retries, transport semantics` — **PASS**
   - No findings.
13. `Websocket surface` — **PASS WITH NOTES**
   - Note: the `ws` feature remains intentionally minimal transport/subscription scaffolding rather than a V1-style reconnecting stack. No release-blocking omission was found within that documented scope.
14. `Security review` — **PASS**
   - No findings.

---

## 3. Verification Captured During Final Broad Audit

**Build / test / lint / doc verification:**
- `cargo check` — PASS
- `cargo check --no-default-features` — PASS
  - Emits only dead-code warnings in the reduced feature set.
- `cargo check --no-default-features --features ws` — PASS
  - Emits only dead-code warnings in the reduced feature set.
- `cargo check --all-features` — PASS
- `cargo test` — PASS
- `cargo test --features ws` — PASS
- `cargo clippy --all-targets --all-features -- -D warnings` — PASS
- `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` — PASS
- `cargo test -- --list` — PASS
- `cargo test --features ws -- --list` — PASS
- `rg -n "https://clob\.polymarket\.com" tests` — PASS
  - No matches.

**Extra tracing redaction spot-checks:**
- `cargo test --all-features deserialize_with_warnings` — PASS
- `cargo test --all-features redact_value` — PASS
- `cargo test --all-features format_value_uses_redacted_payloads` — PASS

**Captured test counts:**
- Default: **126**
- `ws`: **131**

---

## 4. Security Checklist

1. HMAC base64 decode/encode behavior — **PASS**
2. L1 auth typed-data/domain handling — **PASS**
3. Secret redaction in tracing-mode deserialization — **PASS**
4. Secret-bearing response `Debug` redaction — **PASS**
5. HTTPS default with explicit HTTP-only insecure opt-in — **PASS**
6. WSS default with explicit WS-only insecure opt-in — **PASS**
7. Header merge and retry preserve auth plus default transport headers — **PASS**
8. Market-order bounds and type validation — **PASS**
9. 53-bit salt masking for JS-safe transport — **PASS**
10. No `unsafe` blocks under `src/` — **PASS**

---

## 5. Ranked Remaining Issues At Audit Close

At the moment the broad audit closed, the reviewer ranked two remaining low-risk issues:

1. **LOW:** `Cargo.toml:9` still advertised the wrong GitHub repository URL for this Bullpen.fi-owned repo.
2. **LOW:** `src/clob/client.rs:1732` lacked a direct success-shaped retry regression test; direct coverage was stronger on the status-shaped branches at `src/clob/client.rs:1906` and `src/clob/client.rs:1956`.

No medium, high, or critical issue remained.

---

## 6. Post-Audit Cleanup Update

After the final broad audit closed, the two remaining low-risk items were fixed directly in `00a68ff`:

- `Cargo.toml` repository metadata now points at `https://github.com/BullpenFi/rs-clob-client-v2`
- `src/clob/client.rs` now includes a direct regression test for the success-shaped `order_version_mismatch` retry path

**Post-audit cleanup verification:**
- `cargo test retry_order_submission_retries_once_after_success_shaped_version_mismatch --lib` — PASS
- `cargo test retry_order_submission_retries_once_after_version_mismatch --lib` — PASS
- `cargo test` — PASS
- `cargo test --features ws` — PASS
- `cargo clippy --all-targets --all-features -- -D warnings` — PASS
- `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` — PASS

The only complication during local re-verification was environmental: sandboxed local-port binding blocked `httpmock`, so the test-suite reruns were executed outside the sandbox. The code itself did not require further fixes beyond the two low-risk items.

---

## 7. Final Conclusion

After the final broad review and the direct cleanup of its two low-risk polish items, the repository is in a **GO** state.

- No material runtime, parity, or security issue remains.
- The final broad review did not uncover any new blocker.
- The remaining low-risk items identified by that audit have been closed.
- Another broad repo audit is **not necessary**.

A future review should be targeted and only triggered by substantive new code changes.

---

## 8. Archived Final Broad Audit Prompt

The original open `0010` prompt is preserved below for audit traceability.

---

## 9. Why 0010 Exists

`0008` closed **NO-GO** and identified the final concrete blockers.

Those blockers were remediated and then checked in a targeted follow-up under `0009`, which closed **GO**.

`0010` exists because the repository owner wants one more **full-repo Codex audit** across the entire codebase, not because `0009` found a new blocker. This is a final polishing/signoff pass intended to:
- challenge the implementation one last time from end to end
- ensure the Rust crate still mirrors the intended TS V2 behavior closely enough
- confirm the V1-style Rust architecture remains coherent after the late remediation work
- surface any final low-risk cleanup items before release

This plan should be treated as the **last planned broad review pass** unless it finds a new material issue.

---

## 10. Audit Goal

Determine whether current `HEAD` should remain:
- `GO`
- `GO WITH KNOWN LOW-RISK ITEMS`
- `NO-GO`

The primary question is:

**After all prior remediation, does the repo still hold up under one more adversarial full-repo Codex review?**

This audit is broader than `0009`, but it is still expected to be efficient and evidence-driven.

---

## 11. Current Repository Context

Repository:
- `/Users/hongjunwu/Documents/Git/rs-clob-client-v2`

What this repo is:
- Bullpen.fi's unofficial Rust implementation of Polymarket's CLOB V2 SDK
- intended for testing and early Rust adoption
- not an official Polymarket SDK

Current head:
- `7ed79b4` `docs: add plan 0009 targeted audit`

Important context:
- `7ed79b4` is a docs-only commit
- the code remediation under review was implemented in `79332e5` and is present on current `HEAD`
- `0009` already recorded a targeted `GO` result, but `0010` should still verify current `HEAD` directly rather than trusting `0009`

---

## 12. Scope

### In Scope

1. Full-repo review of `src/`, `tests/`, `examples/`, `README.md`, and `Cargo.toml`
2. V1 Rust convention fidelity where those conventions are still applicable to a V2-only crate
3. TS V2 feature and behavior parity across public methods, payloads, signing, auth, and serialization
4. Error handling, retry logic, pagination, and cache behavior
5. Secret handling, transport validation, and logging/redaction safety
6. Test coverage quality and whether important behavior is only validated by inspection instead of direct regression tests
7. Public API polish: whether any remaining sharp edges or inconsistent types should be cleaned up before release

### Out of Scope Unless New Evidence Appears

1. New feature development
2. Major architectural rewrites that would effectively become a `0011` implementation project
3. Reopening closed decisions that were already judged as acceptable intentional drift unless current code demonstrates they are risky in practice

---

## 13. Required Verification Matrix

Run these commands directly:

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
- `rg -n "https://clob\.polymarket\.com" tests`

**Expected current baseline:**
- default test count: **126**
- `ws` test count: **131**
- `rg -n "https://clob\.polymarket\.com" tests`: **no matches**

A mismatch is not automatically a failure, but it must be explained.

---

## 14. Suggested Audit Topology

Use a **full-repo multi-area review**. If the runtime supports subagents, a 2-pass audit with **10 to 16 focused areas** is appropriate. If not, preserve the same areas in sequential review.

### Pass 1 — Rust Architecture / Safety / Maintainability

#### Area 1 — Crate structure, modules, features
**OURS**
- `src/lib.rs`
- `src/clob/mod.rs`
- `src/ws/mod.rs`
- `Cargo.toml`
