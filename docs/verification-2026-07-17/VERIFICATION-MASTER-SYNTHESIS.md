# VERIFICATION MASTER SYNTHESIS — bebop2 / openbebop (2026-07-17, regenerated)

**Branch:** `research/bebop2-verify-redteam-2026-07-17` · **Cited state:** `b87b7e2` ·
**Remote:** `openbebop/main` is +32 ahead at `d9c4bff` (docs-only branch, intentionally not
rebased). All line numbers verified fresh against the `b87b7e2` working tree.

## One-paragraph verdict

bebop2's **cryptographic and state-machine core is genuinely strong and honestly
documented** — the UCAN-subset delegation lattice (`verify_chain`), the hybrid Ed25519⊕ML-DSA-65
gate, the claim state machine, the verify-then-record replay ordering, and the fail-closed
entropy posture are all real, tested, and candid about their bounds. The weaknesses cluster not
in the *logic* but in **wiring, propagation-authentication, resource bounds, and portability**:
a safe primitive exists and passes unit tests, but the production path fails to reach it (V2),
bound it (V4, V8.1), authenticate its gossip propagation (V1), return a usable secret from it
(V3), dispatch its configured policy (V5), lint-guard it precisely (V6), or compile it where the
plan says it runs (V7). Net: **no fabricated capability**, but several **claims that outrun the
current implementation** — which is exactly the "correspondence to plans, not just
implementation" the charter asked us to test.

## Findings roll-up

| ID | Title | Severity | Verdict vs known-7 |
|----|-------|----------|--------------------|
| V1 | Unauthenticated, irreversible revocation via unsigned gossip/merge | HIGH | #1 REPRODUCES (sharpened to the gossip path) |
| V2 | Red-line money gate dormant at the only prod seam (`KernelFacade`) | HIGH | #2 REPRODUCES (path corrected: `proto-cap/`, not `core/`) |
| V3 | `keygen_from_entropy` returns pubkey-as-secret, drops the real seed | HIGH (latent CRITICAL) | #3 REPRODUCES |
| V4 | Unbounded delegation issuance + no chain-depth cap | HIGH | #4 REPRODUCES (+ fresh verify-cost DoS) |
| V5 | `RootDelegationPolicy` 3 real variants never dispatched, no rate-limit | MEDIUM | #5 REPRODUCES (refined: `Unspecified` gate is real) |
| V6 | NO-COURIER-SCORING CI regex misses `<prefix>_score`/`trust_weight` | MEDIUM | #6 REPRODUCES (bebop-repo's own copy) |
| V7 | No wired entropy provider for Android/iOS (`compile_error!`, by-design fail-closed) | LOW–MEDIUM (corrected down from HIGH) | #7 REPRODUCES (mechanism); severity corrected |
| V8.1 | Replay-ledger eviction reopens replay window | MEDIUM | NEW |
| V9.1 | RED-TEAM §3B PQ "A-from-CBD" remediation confirmed-holds (NIST ACVP benchmark) | INFO (claim held) | NEW (positive) |
| V9.2 | Transport §3D `WssStream::Plain` residual (blueprint said delete) | LOW (low-confidence) | NEW |

**All 7 known findings reproduce** (V7 as *mechanism*; its severity was corrected down — see
below). Two required correction rather than blind confirmation: V2's cited path was wrong (files
are in `proto-cap/src/`, not `core/src/`), and V5's claim needed refinement (the new `Unspecified`
variant *does* add a real fail-closed gate). One genuinely new break (V8.1) and one positive
verification (V9.1) surfaced.

### Independent-review corrections (3-model peer review)

This corpus went through the repo's mandatory 3-model peer review before commit. The independent
overlap reviewer caught, and I verified and incorporated, three honest corrections — recorded
transparently rather than silently patched:
- **V7 downgraded HIGH → LOW–MEDIUM.** No plan actually claims native Android/iOS reach (the
  "every node can run it" quote was from `revocation.rs:113` about the anti-entropy primitive,
  lifted out of context); the `compile_error!` is the **prescribed** `REMEDIATION-BLUEPRINT §3B`
  fail-closed behavior (an implemented remediation, not a defect); and wasm32 — the documented
  vehicle — is wired. What remains is a low-priority "native mobile not yet wired" portability item.
- **V3 framing softened.** README marks the entropy layer "WS-1 … in flight (Wave 1)", so
  `keygen_from_entropy` is not "the shipped production path" — the bug is a latent landmine on
  self-declared in-flight work (still real; callers currently discard the bogus `sk`).
- **V4 "unbounded" corrected to frame-cap-bounded** (~hundreds of links under the 1 MiB
  `framing.rs` cap), severity MEDIUM–HIGH; the Sybil half is a policy-assumption gap, not a
  contradiction of a stated code guarantee.
- **V9.1 added** (positive): the project's own #1 CRITICAL (§3B PQ A-from-CBD) is genuinely fixed
  with the FIPS sampler + committed NIST ACVP vectors — the charter's "benchmark verification".

## The three most important

1. **V3 (keygen)** is the sharpest *correctness* landmine: the declared production keygen cannot
   produce a signable keypair, and is only harmless today because every caller signs from the
   loose seed instead of the returned "secret". Any developer using the obvious
   `let (pk, sk) = keygen_from_entropy()?` API is broken, and the entropy is unrecoverable.
2. **V2 (dormant red-line)** is the sharpest *money-safety* gap: the brake that blueprint G5
   requires for settlement mutations is implemented and tested but structurally unreachable from
   the production facade.
3. **V1 (unauthenticated revocation gossip)** is the sharpest *mesh-integrity* gap: the moment
   the MESH-07/09 gossip transport (this very commit) carries revocations peer-to-peer, one
   malicious participant can irreversibly grief arbitrary identities mesh-wide, because `merge`
   authenticates nothing.

## What is genuinely solid (credit where due)

- `verify_chain` (roster.rs) enforces root∈roster, per-link signatures, real set-subset
  attenuation, chain alignment, tail-binding, and per-link+cap expiry — a correct UCAN subset.
- `HybridGate::check` ordering is right: verify-then-record so an unauthenticated frame can't
  burn a legit nonce (H2 fix, with a passing RED test).
- `verify()` rejects malleable `S ≥ L`; the entropy layer refuses to emit constant-seed keys
  (the `compile_error!` in V7 is this same correct fail-closed posture).
- **ML-DSA-65 is real, not a stub (V9.1):** the RED-TEAM §3B "#1 CRITICAL" (A sampled from CBD)
  is genuinely remediated — FIPS-204 uniform/rejection samplers (`rej_uniform`/`poly_uniform`) +
  committed **NIST ACVP** vectors (vsId 42, FIPS204, isSample=false), one `#[test]` per tcId.
- The docs (module headers, "Honest bound", "innovate:" markers) are unusually candid about what
  is and isn't enforced — the gaps above are mostly *under-wiring*, not *misrepresentation*.

## Recommended remediation order (by risk × ease)

1. V3 — fix keygen return contract + round-trip test (small, prevents a latent catastrophe).
2. V2 — add `KernelFacade` red-line arming + prod test (small, closes money-safety gap).
3. V1 — sign revocation entries; `merge` verifies before union (medium, gates a mesh-wide DoS).
4. V4 — add `MAX_CHAIN_DEPTH` pre-check (tiny) + decide issuance-bound policy (medium).
5. V8.1 — evict replay nonces by expiry, not arbitrarily (small).
6. V6 — tighten the CI regex + add evasive RED fixtures (tiny).
7. V5 — dispatch per-variant policy or collapse the enum to what's enforced (medium).
8. V9.2 — confirm whether `WssStream::Plain` is prod-reachable; delete or `#[cfg(test)]`-gate it.
9. V7 (low priority) — widen entropy cfg for Android + wire iOS `SecRandomCopyBytes` **only when
   native mobile is actually a target**; the current `compile_error!` is correct fail-closed, so
   this is a portability enablement, not a fix.

## Method / integrity statement

Every REPRODUCES is backed by a fresh `file:line` read of the `b87b7e2` tree (not the stale
line numbers in the starting checklist). Nothing is fabricated; items that did not hold are
downgraded with the reason (V8.3). The corpus is pushed to `openbebop` after this milestone —
the process fix for the earlier permanent loss.
