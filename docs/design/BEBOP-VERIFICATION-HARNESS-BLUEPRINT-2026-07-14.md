# bebop Verification Harness — blueprint

> **Companion to** `dowiz/docs/design/VERIFIABLE-COGNITION-BLUEPRINT-2026-07-14.md` (§4). This doc carries
> the RED→GREEN detail; it lives in the bebop repo so it sits beside the 13 CI guards it extends.
>
> **Scope:** close the measurement void in bebop2 — the protocol's hardest correctness/security claims are
> currently proven by *prose* or *example tests*, and there is **zero property-based testing, zero fuzzing,
> and zero statistical constant-time analysis anywhere in the tree.** That absence is the harness's mandate.
>
> **Frame:** ground-truth-over-proxy. Every target below is stated as a falsifiable assertion with a RED
> proof that exists *today* and a GREEN condition after the fix. Nothing here is enacted by this document;
> crypto rows are 🔴 red-line → require council + operator.

Status: proposed · created 2026-07-14 · owner SyniakSviatoslav · verified against HEAD ~`c7c6661`.

---

## 1. What is already proven (do NOT rebuild)

| Invariant | Evidence | Verdict |
|---|---|---|
| ML-DSA-65 = FIPS 204, byte-exact | 60 NIST **ACVP** `#[test]`s (`core/src/pq_dsa/acvp_tests.rs`, `kat/acvp/*.json`); uniform-A `rej_uniform` (`pq_dsa.rs:290`) | **PROVEN (external)** |
| ML-KEM core implicit-reject `J(z‖c)=SHAKE256` | `core/pq_kem.rs:987 kem_implicit_rejection_equals_fips203_j`; branchless select `:703` | **PROVEN (core only)** |
| Trust = anchored delegated capability, never reputation | `roster.rs:252 verify_chain`; guard `ci-no-courier-scoring.sh` + RED twin | **PROVEN (structural)** |
| Scope attenuation = real set-subset lattice | `roster.rs:84 Effect::is_subset_of`; `ci-no-flat-scope.sh` | **PROVEN** |
| Hybrid both-legs, no PQ-strip (under `RequireBoth`) | `hybrid_gate.rs:180-206`; real `verify_pq` `signed_frame.rs:229` | **PROVEN (that policy)** |
| verify-then-record replay + bounded nonce set | `hybrid_gate.rs:188-206` (nonce inserted only after all legs verify; `MAX_SEEN_NONCES`) | **PROVEN** |
| Red-line deny-by-default (auth/money/secrets/migrations) | `redline.rs:91`; `ci-no-redline-gate.sh` | **PROVEN** |
| Sovereign core: empty-import, zero-dep, `no_std+alloc` | `verify-empty-imports.sh`; `ci-core-no-ccrypto.sh`; builds `wasm32-unknown-unknown` in CI | **PROVEN (gate CI-only — see T7)** |

bebop2-core is the **embedded reference** (`#![no_std]` + bump-alloc `wasm32-wasip2` in `ports/telegram`)
that the dowiz kernel is not yet — see the master blueprint §6.

---

## 2. The measurement void (the harness's actual job)

Confirmed by grep across the tree:
- **No `proptest` / `quickcheck` / `arbitrary`** — 0 hits in any `Cargo.toml`. Every "property" claim is a
  hand-written example test.
- **No fuzz targets** — no `fuzz/` dir, no `cargo-fuzz`.
- **No statistical constant-time** — CT is asserted by **op-count only** (`sign.rs:973
  scalar_mul_op_count_is_constant`), which proves the *group-layer* branch is gone but **cannot** detect
  data-dependent timing in `mod_l` / `reduce_p` (the C4b hazard).
- **proto-crypto is ~90% skeleton** — `constant_time.rs` / `ladder.rs` / `wycheproof.rs` / `fips_regen.rs`
  are one-field `Placeholder` structs; `lib.rs:80 has_scoring_field()->false` is a tautological label-gate.
- **Two ML-KEM-768 impls** (`core/pq_kem.rs` vs `proto-crypto/pq_kem.rs`) with **divergent correctness**.

---

## 3. Ranked verification targets (security-criticality × un-provenness)

Gate: 🔴 crypto/red-line (council + operator) · 🟡 self-mod-token.

### T1 — `mod_l` / `reduce_p` secret-dependent timing (C4b, HIGH, OPEN) 🔴
- **Where:** `sign.rs:625-637` — `if (byte>>bit)&1` + data-dependent `sub_be` over the SHA-512 nonce hash &
  key; `reduce_p:171` field residual.
- **Why it ranks #1:** biased-nonce → lattice key recovery (same class as the already-closed C4). This is a
  live branch over secret material.
- **RED (today):** no timing test exists. **Build:** a **dudect** harness (Welch's t-test, two input classes
  — fixed vs random secret) asserting `|t| < 4.5` over `sign`. It fails on the current `mod_l`.
- **GREEN:** rewrite `mod_l`/`reduce_p` as fixed-width **Barrett/Montgomery** (no secret-dependent branch or
  memory access); `|t| < 4.5` holds. Add an op-count twin as a cheap CI guard.

### T2 — proto-crypto ML-KEM `H(sk‖c)` wrong implicit-reject 🔴
- **Where:** `proto-crypto/pq_kem.rs:584` returns `SHA3-256(sk‖c)`, **not** FIPS `J(z‖c)=SHAKE256(z‖c)`
  (uses the whole `sk`, not `z`; wrong XOF). Core is correct; this second impl is not — and is
  non-interoperable.
- **RED (today):** no test catches it. **Build:** port `kem_implicit_rejection_equals_fips203_j` to
  proto-crypto — it goes RED on the current impl.
- **GREEN / preferred:** **delete the duplicate impl** (C1 note "keep one") and route proto-crypto through
  `core::pq_kem`. Add a guard: at most one ML-KEM-768 impl in the workspace.

### T3 — CRDT `MerkleLog` convergence (asserted "proven", example-only) 🟡
- **Where:** `sync_pull.rs` — `MerkleLog`, `SyncPeer::{pull,ingest}`, `content_id = SHA3-256(prev‖actor‖seq‖payload)`.
- **RED (today):** convergence is claimed in prose (blueprint:65) but tests are example-based; no
  partition/interleave coverage.
- **Build (property test — shared substrate with the eval harness):** N nodes, a random schedule of
  partition + replay + duplicate-delivery, assert **all nodes reach an identical `root()`** and
  **re-ingesting any frame is a no-op** (idempotence). This is a **graph fixed-point** property — the same
  primitive family as the kernel's convergence math.
- **GREEN:** the property holds over M randomized schedules (seeded, reproducible).

### T4 — Wire `decode_frame` under hostile bytes 🟡
- **Where:** `proto-wire/wire_codec.rs`, `sync_pull.rs:231 from_wire_bytes` — canonicity/fail-closed is the
  load-bearing G1 claim, tested by ~5 hand cases only.
- **Build (`cargo-fuzz`):** target asserting (a) never panics; (b) `decode(x)` then `encode` round-trips to
  a canonical form; (c) non-canonical / malformed input is **rejected**, never silently accepted. This is an
  **injectivity** property.
- **GREEN:** fuzzer runs N iterations with zero panics and zero non-canonical acceptances.

### T5 — Statistical CT for ML-KEM compare / Argon2 / AEAD 🔴
- **Where:** `proto-crypto/constant_time.rs` is a `Placeholder`; the REMEDIATION-BLUEPRINT §3F rung-3
  (dudect) is unbuilt.
- **Build:** make the T1 dudect harness a **reusable rung** applied to every secret-dependent op
  (`ct_eq`, KEM decaps compare, Argon2, AEAD tag check).
- **GREEN:** `|t| < 4.5` for each; wire as a (slow, non-blocking-or-nightly) CI job.

### T6 — `ClassicalUntilPqAudit` PQ-strip acceptance 🟡
- **Where:** `hybrid_gate.rs:180-186` — this policy returns `Ok` on `pq_sig = None` (RequireBoth is proven;
  this branch accepts absent-PQ).
- **Build:** assert production constructs **only** `RequireBoth`; a RED test that an absent-PQ frame under
  the production policy is **rejected**. (Keep the audit policy behind a `dev`/`audit` feature only.)

### T7 — Empty-import + no-alloc hot-path on feature branches 🟡
- **Where:** `verify-empty-imports.sh` + `ci-claim-live-test.sh` fire **only on PR-to-`main`** — a
  feature-branch commit is not empty-import-gated locally. `ARCHITECTURE.md:139` promises a `decide()`
  no-alloc contract that is **unbuilt**.
- **Build:** (a) move empty-import into the per-commit `scripts/law-hooks.mjs` set; (b) build the no-alloc
  assertion — a panicking global allocator installed for a `decide()` test that must **not** allocate; a
  `.text`-size bound guard.

### T8 — G2 dowiz-kernel ↔ bebop2 event-log differential 🟡
- **Where:** `sync_pull.rs:300` (`content_id`) vs dowiz `kernel/event_log.rs` — two content-id producers,
  hand-synced, can silently diverge.
- **Build (differential test):** same canonical input → **identical `content_id`** across both impls. This
  is a **coverage/equality** property; run it in CI on both repos' shared fixtures.

---

## 4. Shared substrate with the eval harness

Targets **T3 (convergence = graph fixed-point)**, **T4 (canonicity = injectivity)**, and **T8
(differential = equality)** are the *same property-based-testing engine* the master blueprint's §3
metamorphic generator uses. Build **one seeded, reproducible property-test crate** (candidate:
`proptest` + a deterministic seed logged per run) and let both the bebop verification harness and the
dowiz eval harness consume it. Crypto CT (T1/T5) is a separate **dudect** crate.

---

## 5. Phasing

| Phase | Targets | Gate |
|---|---|---|
| **B0** (non-crypto, do first) | T3, T4, T7, T8 | 🟡 |
| **B1** (crypto, gated) | T1, T2, T5 | 🔴 council + operator |
| **B2** (policy hardening) | T6 | 🟡 |

**Do first:** T2 is nearly free (port an existing test / delete a duplicate) and closes a HIGH-severity
correctness+interop bug — highest value-per-effort. T3/T4 close the two biggest *un-proven* protocol
claims. T1/T5 are the crypto-timing red-line and must go through council.

**Discipline:** never trust a doc's "DONE" — re-verify against `cargo test`; the corpus has documented
same-day false-greens ("3 rounds of subagents returned false-green — trust `cargo test`, not agent
summaries"). A well-proven FAIL is a successful run.
