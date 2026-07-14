# Fable Adversarial Review — fallacy catalog & fix backlog

> `claude --model fable` read-only adversarial review of bebop (26 files, 24,738 bytes).
> Full output: `/tmp/fable-review-2026-07-10.md`. This doc extracts the falsifiable findings and
> records which are CLOSED (this batch) vs OPEN (proposed backlog, red-line items gated).

## CLOSED this batch (commit f704a30 — verified, RED+GREEN)
| ID | Finding | Fix | Test |
|----|---------|-----|------|
| **B4** | `cost_estimate::route` used a LIFO `Vec` mislabeled "A* binary-heap" + inadmissible `src→i` heuristic → returned suboptimal path (cost 10 vs optimum 2). Replicable-but-wrong. | Real `BinaryHeap<Reverse<Prio>>` + admissible `i→dst` heuristic. | `route_returns_optimal_not_first_popped` (RED: old code returns 10) |
| **B11** | `change_impact` hardcoded `dt=0.05` — the DIVERGENT step the repo's own `dt_corridor` test condemns. Flagship ran in the condemned regime. | Pinned to `0.02` (stable corridor). | reach-floor lowered so stable-dt reach still detected |
| **C2** | `permit_action` gated the RAW effect but shipped the SATURATED (tanh) value → a value could clear the wall raw yet land inside it after saturation; `k` gain silently ignored. | Gates the saturated value; `k` honored. | `permit_action_gates_saturated_not_raw` (RED: old code returned `Some`) |
| **B8** | `vault` salt = `SHA512(pass)[:16]` (not a salt — identical passphrases ⇒ identical keys) + static nonce `[0;24]` (keystream reuse across vaults). | Random salt + random nonce, both stored in `VaultBlob`. | `same_passphrase_vaults_are_distinct` (RED: old code produced identical ciphertext) |

## OPEN — proposed backlog (each has a deterministic follow-up; red-line items need per-change sign-off)
| ID | Finding | Severity | Bucket | Proposed fix |
|----|---------|----------|---------|--------------|
| **B1/B2** | `verify-doc-claims.mjs` check D regex-matches "chacha" → passes contradictory vault claims (scrypt vs Argon2id vs hybrid). | High | doc-gate | Rewrite check D to assert `ml_dsa && ml_kem && argon2 && !scrypt`; add module-comment cross-check. |
| **B5** | `guard::KillSwitch::vote_suspend` accepts unauthenticated voter strings (Sybil). Audit says "consensus met". | High | **red-line (auth)** | Sign votes via `vault::NodeIdentity::verify`; RED: forged-voter vote rejected. OR audit row → "in-process model only". |
| **B6** | `reputation`: suspended courier re-keys (`NodeIdentity::create` free) → returns 0.5 > neutral. "STICKY suspension" vacuous. | High | trust | RED: `rekeyed_suspended_courier_scores_below_neutral` (fails today). Needs external identity-cost or linkage; otherwise audit "Trust: DONE" → "open". |
| **B7** | `wiring`: `l5_applied` computed but has no effect on `proceed`; 2 gates fail open (`contract:None⇒true`, `scope=None,target:Some⇒true`); `io_guard` not imported into `wire()`. | High | **red-line (auth/scope)** | RED: `target_without_scope_refused`; wire `io_guard` or de-audit. |
| **B9** | `matcher::fingerprint` uses `DefaultHasher` (unstable across Rust releases) + compares two instances in the SAME process. | Med | replicability | Use SHA-256 (already in `ledger`); golden-fingerprint fixture test. |
| **B10** | `matcher` quotes DANGER #1 as "which courier, at what price, in what sequence" but assigns `courier: o.src` (pre-chosen). No courier selection. | Med | scope | Implement courier selection + competing-courier RED, or re-scope DANGER #1 to routing only. |
| **B12** | `sandbox` "fail-closed" is a substring denylist (misses `exec 3<>/dev/tcp/...`, `python -c socket`). | High | **red-line (sandbox)** | No-`unshare` + `network=false` ⇒ hard refuse; RED: `/dev/tcp` bypass refused. |
| **B13** | `pod` "no need to trust ... any server" but `self_certify` only checks `id==H(pk)` (any keypair self-certifies). No public-only verifier. | Med | **red-line (identity)** | Add `PublicIdentity` verifier + RED documenting fresh-key self-cert; drop the "trustless" claim. |
| **C1** | `wavefield` reads `BEBOP_WAVE_GATE` env inside a "pure, no RNG/clock" module; `is_ok()` means `=0` enables (broken parse); no shadow stage. | Med | config | Parse `=="1"`; RED: `=0 ⇒ gate off`; implement or drop shadow claim. |
| **C5** | `mapping::live_mapping` rebuilds edges as `LinkKind::Relation` (base 1.0) — `Action` edges silently become `Relation` after one reconnect. | Med | semantics | RED: Action in ⇒ Action out. |
| **C6** | `ledger::open("me",1_000_000)` mints unbalanced money; `conserved()` only reported, not enforced. | High | **red-line (money)** | RED: open-unbalanced ⇒ refuse; enforce paired issuance. |
| **C7** | `zenoh`/`portkey` invoke subscriber callbacks while holding the bus Mutex → re-entrant publish deadlocks (std Mutex non-reentrant). | Med | concurrency | RED: handler calling `publish` on same bus (hangs today); snapshot handlers. |
| **C8** | `rust-core::fexp` range-reduction bound `|r|≤ln2/2` false on negatives (only production use is negative args); test covers only 0/+1. | Low | numeric | RED: `fexp(-13.2)` class test. |
| **C9** | `stabilizer`: φ "optimal branching factor" unproven; `root_locus_poles(-5,0.7,1)` mislabels K<0 as stable. | Low | numeric | Delete optimality prose or benchmark; RED: K<0 ⇒ unstable. |
| **C3/C10** | Audit table cites per-module unit tests as system-wide proof; doc-count drift (README 293 vs audit 292). | Med | doc | Add machine-checked "wired-in?" column; extend doc-claim gate to `docs/design/**`. |
| **A** | `bebop-fundamental-principles-*.md` evidence base is the archived TS tree (no longer built); largest unverified claim in repo. | High | doc | Regenerate against `crates/bebop` or mark §§1,5 historical. |

## Bottom line (fable's own)
The repo's strong habit — small pure modules, fail-closed enums, RED+GREEN tests — is real. But the
**vocabulary of falsifiability is applied faster than the property is established**, and the two
enforcement scripts verify *labels*, not *properties*. Highest-leverage fix: make guardrails check
data-flow + wiring, not words (the repo's own §0 principle, applied to itself). The 4 CLOSED items
above are the first ratchet step in exactly that direction.
