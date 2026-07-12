# bebop2 Remediation — Parallel Execution Plan (2026-07-12)

> Companion to `bebop2/RED-TEAM-REVIEW-2026-07-12.md` + `bebop2/REMEDIATION-BLUEPRINT-2026-07-12.md`.
> Maps the Phase 0–5 blueprint into **independently-buildable workstreams** for max-parallel agent launch.
> Operator rule (2026-07-11): memory-first → push-plans-first → ground-truth-outranks-plans. This doc is the plan.
> Red-line invariant: **NO-COURIER-SCORING** (structural, no rating fields) + **hybrid-only-until-audit** (both sig legs non-`Option`) + **no serde on the signed path** (ARCHITECTURE.md:75).

## 0. Recon — what is ALREADY done (do NOT re-do)

| Item | State | Evidence |
|---|---|---|
| Workspace + `Cargo.lock` | DONE | root `Cargo.toml` `[workspace]` incl. all 4 bebop2 crates; `Cargo.lock` present |
| proto-cap/proto-wire rewrite (replay+expiry-first, hybrid gate) | DONE on `fix/sovereign-core-gate` (unmerged) | 1350 LOC new; `HybridGate::check` checks `is_fresh`+nonce BEFORE classical verify |
| F2 ML-DSA-65 FIPS fix + ACVP | IN PROGRESS (unmerged) on `fix/mldsa-fips204-acvp` | `pq_dsa.rs` 893 lines changed; NOT yet ACVP-anchored (verify subagent spawned) |
| proto-crypto (F1 ladder) | SCAFFOLD on `review/proto-crypto` | `ladder.rs`/`fips_regen.rs`/`wycheproof.rs` stubs |

**Open (not yet touched by any branch):**
- `rng.rs` — still `from_seed` constant; **no entropy source** (F1) → DEADLIEST break.
- `capability.rs` — still signs over `serde_json` (F4 / §4A).
- proto-cap `subject_key` — still self-attested, no `AnchorRoster` (F3 / §3A).
- core numeric: `lyapunov.rs`/`kalman.rs`/`field.rs`/`fft.rs`/`lib.rs`(allocator,fexp)/`algebra.rs`(cosine) — wrong/out-of-envelope (3C).
- CI property gates: real empty-import gate, `deny.toml`, `cargo-audit` (F5 / 3G).
- `proto-wire` plaintext `ws://` accept, no rustls (F6); no channel binding (F7).

## 1. Workstreams (independent builders — parallel-safe)

Each builder works in its OWN `git worktree` (isolated cwd + branch), builds + runs `cargo test`
RED→GREEN, and **does NOT commit** (3-model hook). It reports: worktree path, files touched,
the RED test (fails on current tree) + GREEN test (passes after fix), and `cargo test` evidence.

| WS | Phase | Deliverable | Branch | Blocks | RED (today) → GREEN |
|---|---|---|---|---|---|
| **WS-1** | 0 F1 | Fail-closed entropy source in `core/src/rng.rs`: `Entropy` trait (`getrandom`/`RDRAND`/wasm `crypto.getRandomValues`), ChaCha20 DRBG seeded from it, reseed on fork; `from_seed` → `#[cfg(test)]`/test-only; production keygen returns `Err` if no provider; release profile w/o provider fails to compile | `feat/entropy-fail-closed` | F2/F3/F7 keygen | RED: `keygen([42u8;32])` compiles & silently predicts keys → GREEN: `keygen` requires entropy; constant seed is test-only; same-seed != prod path |
| **WS-2** | 0 F4 | Canonical TLV signing codec `proto-cap/src/tlv.rs`: fixed-layout `DOMAIN_TAG‖struct_tag‖wire_version‖field_count‖[field_id‖u32 len‖bytes]…`, `sha3_256(payload)` as signed field, channel-binding field; per-type domain tags | `feat/tlv-canonical` | F3/F7 | RED: `serde_json` reorder/float silently breaks → GREEN: TLV re-serialize stable; cross-structure reuse rejected by domain tag |
| **WS-3** | 5 3C | Numeric correctness in `core/src`: Lyapunov (Hessenberg+Francis QR, symmetric-only fast-path), Kalman (Potter/Carlson sqrt P=SSᵀ + PSD test), `active_diffuse` sign+CFL+steps guard, non-pow2 Bluestein/panic guard, allocator real-address align+atomic, `cosine_similarity` split-root, `fexp` i64 clamp | `feat/numeric-correct` | — | per-item RED→GREEN (e.g. `[[0,1],[-100,-2]]`→stable; `active_diffuse` energy decays; non-pow2 matches DFT oracle) |
| **WS-4** | 0/3G F5 | Property-gate CI: `deny.toml` (advisories+bans+sources+licenses, ban `openssl-sys`/`native-tls`), `cargo audit` in CI, real empty-import gate (parse RELEASE wasm import section w/ `wasmparser`, fail-closed, RED fixture), `Cargo.lock` `--locked` | `feat/property-gate-ci` | trustable verify | RED: self-captured/trivially-green → GREEN: bad import fixture rejected; advisory DB present |
| **WS-5** | 2 F3 | `AnchorRoster` + UCAN-subset delegation types in `proto-cap/src/roster.rs`: enrolled anchor set, `verify(chain)` enforces root∈roster → chain alignment → `effect ⊆ scope` → `tail.aud==subject_key`; kills self-issue | `feat/anchor-roster` | protocol trust | RED: self-signed cap accepted → GREEN: unknown `subject_key` rejected; scope>effect rejected |

## 2. Verify-in-parallel (not a builder — audits in-flight work)

- **V-1**: review `fix/mldsa-fips204-acvp` for ACVP byte-exactness (uniform A NTT-domain, c̃48, FIPS packing 1952/4032/3309, hint check). External NIST vectors, not self-KAT. → gates F2 "post-quantum" claim.

## 3. Launch order (max parallelism within `delegate_task` batch-of-3 limit)

- Wave 1 (now): **WS-1 + WS-3 + WS-4** (all touch disjoint areas: rng.rs / core numeric / repo infra).
- Wave 2 (after W1 reports, or concurrent second batch): **WS-2 + WS-5 + V-1** (proto-cap tlv / proto-cap roster / F2-verify).
- Wave 3 (sequential gate): F6 rustls + F7 channel binding — depend on WS-2/WS-5, so AFTER they land.

## 4. Integration discipline (3-model review)

For each WS: builder (subagent) → independent REVIEWER subagent (reads diff, security lens) → OVERLAP
subagent (cross-checks reviewer vs blueprint spec). Orchestrator prepares `.review/staged.json`
(builder/reviewer/overlap = 3 distinct agent ids) and commits only after all 3 attest.
NO self-certification. NO merge-to-main without operator sign-off (red-line).

## 5. Honest caveats

- "post-quantum" label stays OFF until WS-1 (entropy) + F2 (ACVP) both green. Until then classical Ed25519 is the only load-bearing sig.
- WS-2/WS-5 make proto-cap a *protocol* (canonical encoding + anchored trust); proto-wire confidentiality (F6) + iroh (F4 mesh) remain after.
- Deleting README claims for absent `reloop/`/`kernel/`/`cli/` once WS-4 doc-truth scan lands counts as progress.
