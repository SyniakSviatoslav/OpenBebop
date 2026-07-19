# AGENTS.md — bebop2 operating rules (binding for every agent/lane)

> Greenfield from-scratch PQ crypto + deterministic core. Zero-dependency, `no_std + alloc`.
> These rules are standing orders; they override convenience and "it's probably fine".

## 0. Global default workflow (multipilot-native, ON by default)
For ANY build task or loop, the default operating mode is a 3-phase pipeline:
1. **FABLE RESEARCH FIRST** — run a claude-fable deep-research pass to produce a *plan blueprint*
   (exact params, algorithm skeleton, function signatures, falsifiable-KAT strategy) BEFORE coding.
2. **3-MODEL DO** — the doer agent/model executes the build. The doer NEVER reviews its own work.
3. **3-MODEL REVIEW/AUDIT (post)** — after finishing, run ANOTHER 3-model review (independent
   reviewer + independent overlap) of the completed task. This is the multipilot native approach.
   **Invariant: doer ≠ reviewer ≠ overlap (no agent/model reviews its own output).**
Override only on an explicit per-task operator instruction.

## 1. Three-model peer review (NEVER self-review) — "threelaterition"
No agent may build AND certify its own work. Every non-trivial change goes through a 3-stage
pipeline; the gate enforces it on commit (`.git/hooks/pre-commit` → `scripts/three-model-review.sh`):

1. **BUILD**   — implementer writes + verifies the code (tests + build green). Does NOT self-certify.
2. **REVIEW**  — a SECOND, independent agent reviews the diff for correctness/security.
3. **OVERLAP** — a THIRD agent (≠ #1, ≠ #2) cross-checks the reviewer against spec/docs, catching
                 shared blind spots where builder & reviewer both assume the same wrong thing.

The §A.3.1 Poly1305 tag was "green" on a roundtrip test that reused the same broken path both ways —
independence of the reviewer is the only reliable antidote. The commit is refused unless BOTH a
distinct `reviewer` and a distinct `overlap` attestation exist, each with a non-empty findings
summary. Builder = reviewer, or reviewer = overlap, fails the gate.

Workflow (builder):
```
bash scripts/three-model-review.sh prepare <builder-id>
# independent reviewer agent:
bash scripts/three-model-review.sh attest reviewer <reviewer-agent> <findings.md>
# independent overlap agent:
bash scripts/three-model-review.sh attest overlap  <overlap-agent>  <findings.md>
```
CI may set `CI_THREE_MODEL_REVIEW=allow` only if it runs its own equivalent review job.

## 2. Verified-by-Math (VbM) — only falsifiable proof validates
A change is validated only if: (1) it works (exercised against reality), (2) it is proven with math
(a deterministic assertion/count with a defined threshold), and (3) the proof is **falsifiable** —
there exists an input under which it goes RED. Ship the RED case alongside the green. A test that
cannot fail is a false-positive metric and does NOT validate. Enforced by
`scripts/guardrail-falsifiable-proof.mjs` (pre-commit) and `scripts/verify-doc-claims.mjs`.

## 3. Red-line areas need per-change confirmation
auth / money / RLS / migrations / bulk-edit / crypto-constant / wire-schema changes are red-line.
Don't silently ship them; flag for human confirmation. **Exception (see §8):** when the red-line
change is fully specified by an approved roadmap plan / blueprint, it may run on autopilot — followed
byte-for-byte, with any error/gap logged to `docs/RED-LINE-LEDGER.md` rather than self-resolved.

## 4. Trust the failing test over the narrative
When a test is red, the bug is real even if the code "looks right". Investigate with an independent
oracle (e.g. a Python reference implementation) before concluding the test is wrong.

## 5. Integration decart — compare & probe before you adopt (operator, 2026-07-14)
**Agnostic, innovative, ethical — zero ideological attachments.** Any NEW integration (a new crate in
`Cargo.toml` [dependencies] · external service/API · transport/provider/carrier/protocol · **or a swap of
one for another**) must FIRST pass a decart evaluation and leave a **decart comparison report** in the
change. No silent adoption. (NOT covered: internal refactors, in-line version bumps within a pinned line,
dev-only tooling that never ships in the sovereign core.)
- Decide by **honest, falsifiable comparison** (in the spirit of §2 VbM — falsifiable evidence over
  narrative) — never by appeal to authority. Modern/Rust-native is the **default + tiebreak**; a proven
  classical method wins **only when honest comparison proves it genuinely better on the merits.**
- The report = a table (candidates × criteria: sovereign-core fit · falsifiable correctness/security ·
  measured performance · supply-chain/license (`cargo-deny`/`deny.toml`) · maintainability ·
  reversibility-as-port · evidence-cited), a `DECISION:` line with a falsifiable reason, an
  **older-as-adapter** note if older tech is kept
  (bridge, **not purged**), and a **mandatory probe** (the strongest honest argument *against* the choice).
- **Banned as a deciding reason:** "industry standard / more mature / battle-tested / community-approved."
  Social proof is not evidence. (An honest *technical* case for a mature tool is welcome — if it wins on
  merit, it's chosen.) Worked example (rustls+ring vs aws-lc-rs vs native-tls) + full template →
  `docs/design/INTEGRATION-DECART-RULE-2026-07-14.md`.

## 6. Sovereign event-exchange (OpenDDE) — enforced invariants
Sovereignty = trust is a **signed, content-addressed, canonically-encoded event any peer verifies from
first principles**, NEVER a reputation score or a blacklist. Provenance + independent verification + capability
scoping — not "rotten-source" filtering (an echo chamber, forbidden by §2 here + `LOGIC-LAWS.md` §4/§23).
Blueprint + principle→code map + gap analysis: `docs/design/SOVEREIGN-EVENT-EXCHANGE-BLUEPRINT-2026-07-14.md`.
These invariants are now MECHANICALLY ENFORCED (were a manual RED-suite):
- **No reputation/scoring of movers** (`scripts/ci-no-courier-scoring.sh`) — pre-commit (`law-hooks.mjs`) + CI.
- **Money/order ⊥ CRDT-merge crate** (`ci-crdt-fence.sh`), **proto-cap ⊥ dowiz-kernel** (`ci-kernel-fence.sh`) — pre-commit + CI.
- **Sovereign core = empty wasm import section / no phone-home** (`verify-empty-imports.sh`) — CI.
- **A DONE/CLOSED mesh claim must cite a live test** (`ci-claim-live-test.sh`) — CI.

## 7. Dependencies track the actual code (operator, 2026-07-19)
Manifests (`Cargo.toml`, `package.json`, any lockfile) MUST reflect what the code actually
imports and what the registry actually publishes — **derived from ground truth, not from a
pasted status or an aspirational pin.** Before adding/bumping/keeping a dependency:
- Verify the crate/package + version **exists on the registry** and **resolves** (a pin to a
  non-existent version — e.g. `@cloudflare/workers-types@^4.20260714.0` when the highest real
  publish is `4.20260702.1` — is a real defect: every fresh clone and Dependabot run fails).
- Verify the code **actually uses** the dep. An unused dep is removed, not carried. A used-but-
  unlisted dep is added. Duplicate implementations (e.g. a TS worker duplicating a native Rust
  daemon) get the redundant side removed, not both maintained.
- This is a VERIFY gate, not a courtesy: the manifest is downstream of the code, never the other
  way around. (NOT a decart trigger by itself — §5 still governs *new* integrations / swaps.)

## 8. Red-line work on autopilot: blueprint byte-for-byte, zero самодіяльність (operator, 2026-07-19)
Red-line areas (§3: auth / money / RLS / migrations / bulk-edit / crypto-constant / wire-schema)
MAY now be executed on autopilot **without per-change confirmation — but ONLY when driven strictly
by an approved roadmap plan or blueprint.** This is the one place where §3-Ground-truth / §2-VbM
"figure it out from the live repo" does **NOT** apply. Instead:
- **The blueprint's exact schema is authoritative and followed byte for byte.** Zero inventions,
  zero discoveries, zero improvisation (жодної самодіяльності). If the spec gives exact bytes,
  field order, or constants, they are reproduced exactly — the agent does not "improve" or
  re-derive them.
- **No blueprint coverage = no autopilot.** If the red-line change is not fully specified by an
  approved plan/blueprint, it reverts to §3 (human confirmation required). Autopilot is licensed
  by the blueprint, not by the red-line classification.
- **Errors, ambiguities, and suggestions are NOT acted on — they are logged.** Any defect found,
  gap in the spec, or improvement idea is written to `docs/RED-LINE-LEDGER.md` (append-only) and
  left for the planning team. The executing agent continues only on the byte-exact, unambiguous
  parts, and stops if the blocker is load-bearing. It does not self-resolve a red-line ambiguity.

## Build/test
- `cargo test` — 914 Rust tests, RED+GREEN, 0 fail
- `cargo test -p bebop2-core` (full suite), `cargo clippy -p bebop2-core --all-targets`
- Crypto KATs live in `bebop2/core/src/kat/`; RFC 8439 §2.5.2 + Appendix A.3 are the Poly1305 anchors.

---

## Operating rules — memory-first + push-plans-first (operator, 2026-07-11)

1. **Update living memory FIRST.** Before writing/planning any code, record new changes, plans,
   decisions, and ground-truth facts to the canonical corpus. Source of truth = the corpus, not chat.
   - bebop/bebop2 (protocol) → `/root/.claude/projects/-root-bebop-repo/` corpus.
   - dowiz (product) → `/root/.claude/projects/-root-dowiz/memory/MEMORY.md`.
2. **Push plans to remote FIRST.** Plans/roadmaps/decision docs are committed + pushed to `origin`
   before execution — so they can never be lost to a crashed session or stale context.
3. **Ground truth outranks plans.** Re-verify code claims (`grep`/`git`/`cargo test`) before trusting a
   pasted "verified" status. Plan = desired state; live repo = what IS. Keep DONE (verified) vs PLANNED
   separate. **Both artifacts are maintained (2026-07-12 operator directive):** `bebop` is the live
   Rust coding-agent CLI (`crates/bebop/`); `bebop2/` is the from-scratch, zero-dep, FIPS 203/204
   protocol (ACVP-verified, canonical TLV, rustls — see `docs/design/BEBOP-CLAIM-AUDIT-2026-07-12.md`).
   The old "bebop is PARKED as a protocol" note is RETIRED — both ship.
4. **Structure before code:** PARALLEL-SAFE (independent files, zero-pivot, non-red-line → own branch)
   vs SEQUENTIAL GATES (red-line, external validation, tier deps). Shared Tier spine with dowiz.
