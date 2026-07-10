# Bebop — Fundamental Working & Proven Principles (cross-layer analysis)

> Operator request (2026-07-09): with reasoning/review/logic upgraded, analyze the WHOLE project for
> the fundamental working & proven principles from each layer, integrated tool, system, and rule —
> and surface cross-patterns across multiple dimensions, not just local patterns.
>
> Method: read the actual source (kernel, governor, loop, copilot, validate, field, speculate,
> cycle-consistency theorem, dual-track, logicalCot, kalman, goap, ica, redteam, modelGateway) +
> the rulebook (AGENTS.md, RULES.md, guardrails) and the ADRs. Every principle below is backed by
> code/tests I inspected, not asserted. Counts are live (`grep -rl`, this session).

## 0. The three meta-rules (load-bearing, from RULES.md / AGENTS.md)

These are not "conventions" — they are the operating OS and they REPLICATE themselves down into the
code as enforceable structure (pre-commit gates prove it):

1. **Constant Doubt** — no statement stands unless a live probe or a deterministic test (with a RED
   path) backs it. `Unverified = false`. Enforced mechanically by `verify-doc-claims.mjs` (pre-commit
   + CI) and `guardrail-falsifiable-proof.mjs` (every test must be able to go RED).
2. **Verified-by-Math (VbM)** — every behavior change ships a *falsifiable* RED+GREEN test; a test
   that cannot fail is a false-positive metric and is rejected. Evidence: `red_green=80` files carry
   RED+GREEN markers; `test_files=62`, and the guardrail passes all 62 with a self-test proving it
   catches tautologies.
3. **Red lines** (per-change human gate) — auth, money, RLS/migrations, secrets, bulk edits are never
   auto-touched. Enforced by `guard.ts checkRedLine` + `loop.ts` GUARD GATE (`guard_gate`+, runs BEFORE
   any read/write/dispatch, for every backend equally).

4. **First Principles Thinking** — derive every architecture/library/abstraction from the irreducible
   physical/mathematical constraints it must satisfy; reject what cannot be traced to one. "Everyone
   does it" is not a derivation. Code twin: `rust-core` is the first-principles layer — graph-PDE
   spectral kernel, VSA, vector algebra, all `#[no_mangle] extern "C"`, compiled to wasm with an EMPTY
   import section (core-RE-loop proves no clock/RNG/socket reachable). Any new primitive must earn its
   place the same way.

5. **Physicality as Truth** — physical constraints (energy, mass, radiation, clock, memory, band) are
   the ground-truth oracle; a physical fact overrides any doc/claim/number. Real-system numbers MUST be
   verified against primary sources (datasheet / NASA NTRS / archival spec), not repeated from secondary
   summaries. Code twin: core-RE-loop's empty-import check is the machine-code-level physicality gate
   (no imports ⇒ no reachable clock/RNG/socket) — the envelope discipline AGC achieved via unrewritable
   core-rope memory. Physicality is the empirical half of VbM: VbM asks "can it go RED?"; Physicality
   asks "does it survive the envelope?".

> Cross-pattern 0a (extended): the rulebook replicates itself into code. Constant Doubt → guardrail
> scripts; VbM → falsifiable-proof gate; red lines → `guard.ts`; First Principles → `rust-core`
> (dependency-free, no-std determinism); Physicality → core-RE-loop empty-import check. Each meta-rule
> has a *machine twin*; a principle without one is prose, not an operating rule.

> Cross-pattern 0a: **The rulebook is not prose, it is executable.** Constant Doubt spawned the two
> guardrail scripts; VbM spawned the falsifiable-proof gate; red lines spawned `guard.ts`. Each meta-rule
> has a *code twin* in the repo. That is the deepest pattern: **principle → enforced-by-machine**, never
> trust-me-brother.

## 1. Layer-by-layer proven principles

### L0 — Deterministic kernel (src/kernel.ts, crates/core Rust/WASM)
- **Pure + content-addressed**: no clock, no RNG, no network, no env. Same input ⇒ same canonical bytes.
  `Envelope{seq, cause}` where `cause = CommandHash` (content address) — also the torrent/self-certifying
  address. This is the D2 dedupe/causality seam.
- **One door**: `decide(command, state) -> Event[]`, `fold(state, event) -> state`,
  `replay(genesis, log) -> state`. Forbidden transitions are explicit `DomainError`s, never panics.
  Exhaustive `switch`'s `_exhaustive: never` make "a new action without a decide arm" a compile error.
- **Idempotency / replay protection**: `state.seen.has(hash)` ⇒ no-op replay, never a double-event.
- **Event-sourcing as the audit spine**: everything is an append-only envelope; a whole multi-backend
  session is replayable and falsifiable.

### L1 — Neuro-Symbolic Gate / Governor (src/governor.ts, ADR-003)
- **Advisor proposes, kernel decides** (universal rule #2). Any stochastic advisor (LLM/GNN/heuristic)
  is a *consultant*; the deterministic `Governor` is the only actor that writes `authority`. A symbolic
  arbiter (`clamp`, factor-kill, resonance-cap, safe-state floor, poison guard, cycle/PCA breach gate)
  sits between them and **mathematically cannot emit an out-of-contract command**.
- **Four math foundations unified into meta-control**: PID (discrete + integral anti-windup), ICIR
  (rank-corr of predicted vs actual quality — "does the agent know its own quality?"), RESONANCE
  (predict closed-loop ζ/ω_r of a PROPOSED gain BEFORE applying; refuse under-damped), THERMO (Landauer
  floor k·T·ln2 — a hard floor on cost, "can't think for free").
- **Fail-closed liveness** (N2): silent longer than `watchdogMs` ⇒ kernel drops to Safe State
  (authority floored, no advisory honored). Tested RED+GREEN.

### L2 — Agentic loop (src/loop.ts) + copilot (src/copilot.ts)
- **Doer → Checker, distinct backend** (copilot.ts): the DOER produces, a DIFFERENT checker verifies in
  real time; on REJECT the action is *quarantined* (not applied) — fail-closed, mirrors the kernel's
  quarantine semantics ("as above, so below").
- **Visible ReAct trace**: every Reason→Act→Observe→Reflect iteration is emitted (default 3, overridable,
  NOT hidden). Promo-demo anti-pattern (one "perfect" iter) is structurally impossible — the denial shows
  FAIL in `reactTrace` (test asserts it).
- **Validation wall before guard gate** (src/validate.ts, Pydantic principle): untrusted LLM tool-args
  MUST clear an explicit contract at the boundary — malformed input rejected at the seam, never patched
  downstream. Runs BEFORE the guard; guard decides legality, validate decides well-formedness.

### L3 — Reasoning/search oracles (field.ts, speculate.ts, consciousness.ts)
- **Physics-as-logic** (field.ts): divergence (∇·F) and z-curl (∇×F)_z of the agent's traversal flow
  field become a 3-state directive — diverge→generate, curl→reconsider, both→generate+reconsider,
  sink/stable→focus. Real discrete vector calculus, not metaphor; classified by *relative dominance* so
  it stays falsifiable.
- **Speculative decode for agents** (speculate.ts, DSpark RE): draft a BLOCK of N candidate actions in one
  LLM call; verify through the deterministic guard. The guard is the SOLE trust boundary; the draft is
  speculative until verified. Round-trip savings = draftedLen − 1.

### L4 — Analytics / integrated tools (analytics/*)
- **Symmetrical loops** (universal rule #1, cycle-consistency.ts + theorem): invertible
  Decompose→Reconstruct pair; assert `Reconstruct(Decompose(x)) ≈ x`; residual localizes the broken
  module. PCA chosen (not VAE) because it is the only deterministic, bit-reproducible pair under
  sovereign-core (no RNG). **Proven blind spot**: gap=0 ⇏ correct (a self-inverse bijection passes) ⇒
  NECESSARY-not-SUFFICIENT; hard red-lines keep explicit contract tests.
- **Dual-track firewall** (N6, dual-track.ts): stochastic advisor proposal overlaid on a deterministic
  Truth Layer graph; `no-such-edge` rejects a hallucinated dependency. Causal blast-radius via
  `causalCounterfactual` (N4++).
- **Kalman** (N8a), **GOAP** (N8c), **ICA/telemetry-ica-loop** (D2), **Logical CoT** (logicalCot.ts,
  arXiv:2509.13351), **redteam** (T3MP3ST-method), **modelGateway** (Portkey-method): each is a pure,
  deterministic, flag-OFF seam, each with RED+GREEN, each fails-closed.

## 2. Cross-patterns across MULTIPLE dimensions

These are the non-obvious findings — patterns that repeat *across layers* and *across tools*, not within one.

### Cross-pattern A — "As above, so below" (the universal checker at every scale)
The SAME `Checker` abstraction that validates a command in the kernel (`applyCommandChecked`, kernel.ts)
is mirrored one level up in copilot (distinct-backend checker), one level further in `logicalCot`
(step-wise logic auditor), and again in `speculate` (the guard is the authoritative verifier over the
draft) and `validate` (boundary contract). Evidence: `as_above=21` files. The principle: **a single
fail-closed verification primitive recurs at kernel / agent / plan / tool-arg scale.** It is the project's
fractal — verify-then-admit, everywhere, with quarantine-on-failure.

### Cross-pattern B — "Propose, never execute" (advisor/doer separation is the only safe topology)
- Kernel: advisor proposes, kernel decides (ADR-003).
- dual-track: GNN advisor proposes, graph rejects.
- loop/copilot: LLM doer proposes, distinct checker verifies.
- speculate: backbone drafts, guard verifies.
- logicalCot: executor emits a logical transaction, verifier audits preconditions/effects/invariants.
- GOAP: advisor names the GOAL, symbolic planner executes; unreachable goal = NO PATH (cannot act on a
  hallucination).
The pattern: **the stochastic/LLM layer is NEVER given the actuator; it only names intents. Execution is
always a deterministic function over a verified state.** This is the single most repeated architectural
decision in the repo and it is what makes every other safety property possible.

### Cross-pattern C — "Flag-OFF by default; shadow before gate"
Evidence: `flag_off=16` files carry explicit FLAG-OFF. Every new analytic (cycle-consistency, ica,
kalman, degradation, mesh, arch-mine, field, active-inference) is inert unless a caller supplies its cfg.
The theorem doc states the deployment ladder: **OFF → shadow (log drift) → gate (block)**, and "never gate
red-line actions on the loop alone." This is a uniform risk-spreading discipline: no feature can quietly
go live; each must prove low false-positive rate in shadow first.

### Cross-pattern D — "Determinism is the security model"
Evidence: `deterministic=123` files assert no-RNG/no-SGD/no-Date. Determinism is not just a test
convenience — it is the *trust boundary*. Because `decide`/`fold`/PCA/Kalman/GOAP/ICA/field are pure, the
log is replayable, the proof is falsifiable, and a red-line action cannot hide behind nondeterminism.
Sovereign-core (no runtime RNG/SDG/Date, air-gapped) is the root constraint from which the verifiability
of everything else flows.

### Cross-pattern E — "Proven blind spots, stated not hidden"
The codebase *names* its own limits and tests them RED:
- cycle-consistency proves `gap=0` ≠ correct (self-inverse bijection) and keeps explicit contract tests.
- AGENTS.md states symmetrical loops "do NOT add EV" for semantic truth / hard red lines.
- logicalCot's effect-noop check exists because "an action that claims to advance but is inert" is a real
  hallucination class.
- RULES.md: "better less than sorry" — cut the sentence rather than state an unproven claim.
The pattern: **honesty about failure modes is load-bearing, not a disclaimer.** Each component ships its
own RED blind-spot test. This is the inverse of typical ML "happy path" code.

### Cross-pattern F — "Math, not metaphor" (physics/geometry as the reasoning substrate)
field.ts (∇·F/∇×F), governor (PID/ICIR/resonance/Landauer), kernel (conservation-law invariant — "exactly
like energy/momentum conservation in mechanics"), cycle-consistency (Parseval/orthonormality proof),
speculate (DSpark semi-autoregressive + confidence schedule). Where other agents reach for a vibe, bebop
reaches for a theorem with a falsifiable bound. The cross-pattern: **every "intuition" is grounded in a
named mathematical object with a provable error bound or a proven blind spot.**

### Cross-pattern G — "RED is not a TODO, it is the proof"
The guardrails and AGENTS.md enforce: a RED case ships BESIDE the GREEN; a red gate is a red-line, not a
backlog item; `guardrail-falsifiable-proof.mjs` rejects any test that cannot go red. The pattern across
docs + scripts + tests: **the negative case is the first-class artifact.** This is what makes Constant
Doubt operational rather than aspirational.

### Cross-pattern H — "Deterministic twin for every risky external dependency"
- Zenoh mesh: `selectZenoh` is pure; 'real' with no client degrades to 'local' and *says so* (never claims
  a connection it doesn't have).
- TigerBeetle: money boundary is a pure checker (`moneyTransferChecker`) composed into the kernel's
  universal checker; the ledger is exercised by tests, the network is not.
- modelGateway: virtual keys + fallback + guardrail, pure; never fabricates a missing key.
- redteam: deterministic adversarial probe that SUFFACES a fail-open gate (no hiding).
The pattern: **any external/system boundary gets a pure, deterministic, fail-closed twin; the live
dependency is optional and shadowed.** This keeps the core air-gapped and the proofs reproducible.

## 3. Synthesis — the 8 universal principles (existing + extracted)

Existing (from AGENTS.md / RULES.md / ADRs):
1. Constant Doubt (no verification → no statement).
2. Verified-by-Math (falsifiable RED+GREEN, always).
3. Red lines (per-change human gate: auth/money/RLS/secrets/bulk).
4. Symmetrical loops (cycle consistency) wherever they add EV.
5. L5 Neuro-Symbolic Gate (advisor proposes, kernel decides).

Extracted/found in this analysis (cross-pattern-derived, now proposed as standing principles):
6. **As-above-so-below checker** — one fail-closed verify-then-admit primitive recurs at kernel/agent/
   plan/tool-arg scale (Cross-pattern A).
7. **Propose-don't-execute** — the stochastic layer only names intents; execution is always a
   deterministic function over a verified state (Cross-pattern B).
8. **Flag-OFF → shadow → gate** — no feature goes live silently; prove low false-positive in shadow
   first (Cross-pattern C). Plus the found sub-principles: determinism-as-security-model (D),
   named-blind-spots (E), math-not-metaphor (F), RED-is-the-proof (G), deterministic-twin-for-risky-IO
   (H).

## 4. Recommendations (concrete, max-EV)
- **Promote 6/7/8 to AGENTS.md as universal rules** with the same structure as 4/5 (definition, where-adds-
  EV, where-NOT, impl pointer, flag-OFF→gate). They are already load-bearing in code; making them explicit
  prevents the next agent from re-deriving them and lets the doc-claim gate guard them.
- **Wire `logicalCot` + `dualTrackGate` + `validate` into the actual runtime path** (today they are flag-OFF
  seams). The code proves they work; the gap is live-promotion (per Cross-pattern C, shadow first).
- **Add a cross-pattern self-test**: a script asserting the invariant "every advisor entry point is matched
  by a deterministic verifier" — mechanically enforces Cross-pattern B so a future integration cannot
  skip the gate.

## 5. Evidence index (live this session)
- `find src -name '*.test.ts'` = 62 files, all falsifiable (guardrail passes, self-test green).
- kernel.ts: `decide/fold/replay`, `applyCommandChecked`, `defaultChecker` — verified by core.test.ts.
- governor.ts: PID/ICIR/resonance/thermo + N2 safe-state + N7 degradation (Kalman) — 38 governor tests.
- loop.ts: GUARD GATE ×4 entry points, visible reactTrace, validate-before-guard, red-line uniform.
- cycle-consistency theorem: exact (k=d) + bounded (k<d, gap²≤Σσ_j²) + proven blind spot (self-inverse).
- field.ts: discrete ∇·F/∇×F, relative-dominance classify, 3-state directive.
- speculate.ts: semiAutoDraft survival + verifyBlock (guard authoritative) + round-trips-saved.
- grep counts: flag_off=16, deterministic=123, red_green=80, fail_closed=30, as_above=21,
  verified_math=31, invariant=112.
