# bebop Verification Harness — WAVE PLAN (execution)

> Companion execution plan for `BEBOP-VERIFICATION-HARNESS-BLUEPRINT-2026-07-14.md`.
> Push-plans-first: this plan is committed + pushed BEFORE code. Ground-truth-over-proxy:
> every wave lands a RED proof first, then GREEN. Crypto rows are 🔴 red-line → PAUSE for
> council + operator; this agent executes only 🟡 non-crypto waves autonomously.

Status: in-progress · created 2026-07-14 · owner SyniakSviatoslav · branch `feat/logic-governance`

---

## Ground-truth corrections to the blueprint (verified against HEAD 4431c59)

| Blueprint claim | Ground truth | Effect |
|---|---|---|
| `sync_pull.rs` in `proto-cap` | actually `bebop2/proto-wire/src/sync_pull.rs` | path fixed; T3 valid |
| T8 differential vs `dowiz kernel/event_log.rs content_id` | **dowiz kernel has NO `content_id` producer** (grep empty) | T8 premise stale → **DEFER** until a kernel producer exists |
| T7 no-alloc `decide()` in bebop | **no `fn decide` in bebop2** (it's a dowiz-kernel contract) | T7 no-alloc part N/A here; only the empty-import-per-commit part is actionable |

---

## Wave sequence

| Wave | Target | Gate | Lane | RED → GREEN |
|---|---|---|---|---|
| **W1** | T3 — MerkleLog convergence (graph fixed-point) | 🟡 | auto | property test: N nodes, random partition+replay+dup schedule → identical `root()` + re-ingest is no-op |
| **W2** | T4 — `decode_frame` canonicity/injectivity under hostile bytes | 🟡 | auto | property/fuzz: never panic · decode→encode round-trips canonical · non-canonical rejected |
| **W3** | T7 — empty-import gate per-commit | 🟡 | auto | move `verify-empty-imports.sh` into `scripts/law-hooks.mjs`; RED = a stray import on a feature-branch commit trips locally |
| **DEFER** | T8 — kernel↔bebop content_id differential | 🟡 | blocked | no kernel `content_id` producer yet; revisit when it lands |
| **PAUSE** | T1/T2/T5 — mod_l/reduce_p timing, proto-crypto H(sk‖c), statistical CT | 🔴 | council+operator | dudect harness + Barrett/Montgomery rewrite + duplicate-impl delete — NOT this agent's hands |
| **W-last** | T6 — ClassicalUntilPqAudit PQ-strip acceptance | 🟡 | auto | assert production constructs only `RequireBoth`; RED = absent-PQ frame under prod policy rejected |

---

## Decart report — property-testing dependency (required by Integration Decart Rule)

New integration: a property-based-testing crate (W1/W2 need randomized, seeded, shrinking generators).

| Candidate | bare-metal fit | falsifiable correctness | perf | supply-chain/license | maintainability | reversibility-as-port | evidence |
|---|---|---|---|---|---|---|---|
| **proptest** | dev-dep only (never in `[dependencies]`; zero runtime/wasm impact) | shrinking + seeded reproducibility (persists failing seed) | fast enough for CI | MIT/Apache-2.0, widely-vetted, `proptest-derive` optional | active, ergonomic strategies | tests are plain `#[test]` fns; removing the dep = delete the test module | rust-fuzz ecosystem standard for stateful/shrinking |
| quickcheck | dev-dep only | weaker shrinking, no seed persistence | fast | MIT/Apache | simpler but less expressive | same | older, less maintained |
| hand-rolled LCG generator | zero dep | no shrinking, we own the RNG | fast | none | we maintain it | trivial | full control but reinvents shrinking |
| cargo-fuzz (libFuzzer) | separate `fuzz/` crate, nightly | coverage-guided, best for T4 byte-injectivity | slow (long-running) | Apache/MIT | nightly toolchain | isolated crate | best for hostile-bytes |

DECISION (UPDATED after W1 ground-truth, 2026-07-14): **hand-rolled xorshift64 seeded RNG, zero new
dependency.** W1 (T3) was implemented WITHOUT proptest — a deterministic, seeded, shrinking-free property
harness. Falsifiable reason: proto-wire is a zero-dep / offline-clean crate (the sovereign-core gate in
blueprint §1 forbids a dev-tree leaking into the shipped graph); the convergence property needs only
*reproducible randomized schedules + a logged failing seed*, which xorshift gives with zero transitive deps.
proptest's shrinking is unnecessary for this integer/enum schedule space and its `rand`/`bit-set` dev-tree
is a real cost for no benefit here. The PROBE in the original report was decisive.
For T4's raw hostile-bytes injectivity, **cargo-fuzz** in a separate `fuzz/` crate (nightly, non-blocking CI)
remains the right tool — coverage-guided beats random for that surface, and it is fully isolated.
Older-as-adapter: n/a. Reversibility: the harness is plain `#[test]` + a local fn; removing it is a one-file
patch with zero footprint.
upgrade trigger: if a future wave needs shrinking of *arbitrary structured* inputs (not integer/enum
schedules), adopt proptest as dev-only — still guarded by `ci-core-no-ccrypto.sh`.

---

## Discipline (from blueprint §5, binding)

- Never trust a doc "DONE" — re-verify with `cargo test`. A well-proven FAIL is a successful run.
- RED must exist and fail on today's code before GREEN.
- 🔴 crypto waves are NOT executed autonomously — plan them, stop, escalate to council+operator.
- Every wave: commit only when green; push after each wave (push-plans-first + keep remote current).
