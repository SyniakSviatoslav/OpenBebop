# bebop2 / openbebop — Verification & Red-Team Corpus (2026-07-17, regenerated)

> **Branch:** `research/bebop2-verify-redteam-2026-07-17` (worktree of `/root/bebop-repo`).
> **Checked-out state cited throughout:** `b87b7e2` ("MESH-07 sync-pull frame + MESH-09
> iroh-transport body + W4-3 anti-entropy integration").
> **Remote delta note:** `openbebop/main` has advanced **+32 commits** to `d9c4bff`
> (a P06 hybrid Ed25519⊕ML-DSA-65 K/V signer merge) since this branch's base. This is a
> **docs-only research branch**; per the research charter it is **not** rebased. Every line
> number below was re-verified fresh against the `b87b7e2` working tree — do not assume they
> hold on `d9c4bff`.

## Why this corpus was regenerated

An earlier run of this exact research produced a corpus in a worktree at
`/root/bebop2-verify-redteam` that was **never pushed to a remote** and whose local directory
later vanished. A git-archaeology pass (reflog, `fsck --unreachable`, `rev-list --objects
--all`) found **zero trace** of the lost text. Root cause: *work committed only to a local
worktree that was then destroyed before any push*. This regeneration is a **fresh re-audit
against current code**, not a reconstruction — and it is pushed to `openbebop` after each
milestone. The load-bearing process fix is the push cadence, not the prose.

## Scope (operator charter, verbatim)

> "хитрі та справді непрості тести, ін'єкції, red teaming, перевірки бенчмарків та різних edge
> cases, реальна відповідність заявленому у планах (а не поточній імплементації) — спочатку усе
> лише bebop2 & openbebop, а також документацію та вікі."

Tricky/non-trivial tests, injections, red-teaming, benchmark verification, edge cases, and
**real correspondence to what is CLAIMED IN PLANS** (not merely to the current implementation)
— bebop2 & openbebop only, plus docs/wiki.

## Findings index

| ID | Title | File(s) | Severity | Known-7? | Verdict |
|----|-------|---------|----------|----------|---------|
| [V1](V1-unauthenticated-irreversible-revocation.md) | Unauthenticated, irreversible revocation via unsigned gossip/merge | `proto-cap/src/revocation.rs` | HIGH | #1 | REPRODUCES (sharpened) |
| [V2](V2-dormant-redline-money-gate.md) | Red-line money/settlement gate is dormant at the only production seam | `proto-cap/src/hybrid_gate.rs`, `proto-cap/src/facade.rs` | HIGH | #2 | REPRODUCES (path corrected) |
| [V3](V3-keygen-destroys-secret-key.md) | `keygen_from_entropy` returns the public key as the "secret" and discards the real seed | `core/src/sign.rs` | HIGH (latent CRITICAL) | #3 | REPRODUCES |
| [V4](V4-unbounded-delegation-and-chain-depth.md) | Unbounded delegation issuance + no chain-depth cap (Sybil + verify-cost DoS) | `proto-cap/src/roster.rs` | HIGH | #4 | REPRODUCES (sharpened) |
| [V5](V5-rootdelegationpolicy-pure-marker.md) | `RootDelegationPolicy` — 3 real variants never dispatched, no rate-limiting | `proto-cap/src/node_id.rs` | MEDIUM | #5 | REPRODUCES (refined) |
| [V6](V6-ci-no-courier-scoring-regex-gap.md) | NO-COURIER-SCORING CI gate misses `<prefix>_score`/`trust_weight` compounds | `scripts/ci-no-courier-scoring.sh` | MEDIUM | #6 | REPRODUCES |
| [V7](V7-mobile-compile-target-failure.md) | No wired entropy provider for Android/iOS (`compile_error!` — by-design fail-closed) | `core/src/rng.rs` | LOW–MEDIUM (corrected down) | #7 | REPRODUCES (mechanism); severity corrected |
| [V8](V8-additional-findings.md) | Additional: replay-ledger eviction reopens replay window; plans-vs-impl notes | `proto-cap/src/hybrid_gate.rs` + docs | MEDIUM | new | NEW |
| [V9](V9-benchmark-and-remediation-verification.md) | Benchmark verification: RED-TEAM §3B PQ/ACVP "A-from-CBD" remediation confirmed-holds; §3D `Plain` transport residual | `core/src/pq_dsa*`, `proto-wire/src/wss_transport.rs` | INFO / LOW | new | NEW |

See **[VERIFICATION-MASTER-SYNTHESIS.md](VERIFICATION-MASTER-SYNTHESIS.md)** for the bebop2/openbebop
roll-up, and **[CROSS-REPO-VERIFICATION-MASTER-SYNTHESIS.md](CROSS-REPO-VERIFICATION-MASTER-SYNTHESIS.md)**
for the five-repo view.

## Method notes

- Every "REPRODUCES" is backed by a `file:line` citation read fresh from the `b87b7e2` tree.
- Where a known finding's cited path was wrong (V2 was cited under `core/src/` but lives in
  `proto-cap/src/`), the correction is stated plainly rather than silently patched.
- Nothing here is fabricated. Where a claim did not hold, it is downgraded with the reason.
- Several findings are **latent** (correct-but-dangerous, or dormant): they are not currently
  exploited because callers happen to avoid the trap, but the trap is armed in the code.
