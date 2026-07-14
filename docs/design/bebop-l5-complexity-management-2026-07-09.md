# Bebop L5 — Complexity Management, not Physics (algorithm inventory reframed)

> Operator directive (2026-07-09): if we build an L5 agent for LOGISTICS / order-hub control (not a
> physical drone), the "9 algorithms" become **complexity-management tools**, not physics tools. Map
> each to bebop's existing deterministic twins, drop the bloat, and diagnose the real hole. Then extend
> the tensor/multipilot directive across reasoning/review/governor/testing.

## 1. The inventory reframed (algorithm → bebop deterministic twin)

| # | Algorithm (intent) | L5 hub role | Bebop twin (already built) | Sovereign-core verdict |
|---|---|---|---|---|
| 4 | Transformer (task decomposition) | Reasoning engine: order → atomic events | `loop.ts` ReAct + `logicalCot.ts` PDDL-INSTRUCT proof; `field.ts` ∇·F/∇×F search | KEEP — the LLM is the *doer*; logicalCot is the deterministic verifier |
| 7 | GNN (graph relation / bottleneck) | Mapping: order/role/delivery graph | `arch-mine.ts` (import+wikilink adjacency → SVD/PCA latent clusters) + `dual-track.ts` Truth Layer | KEEP + UPGRADE — our graph understanding is SVD-based, not learned; the **hole** |
| 5 | GAN (synthetic chaos for stress-test) | Self-testing: generate impossible orders | `redteam.ts` (T3MP3ST-method deterministic mutation probe) | KEEP — deterministic mutation > trained GAN (no SGD) |
| 6 | VAE (anomaly / reconstruction error) | Integrity: "logic broken" sentinel | `cycle-consistency.ts` (PCA round-trip = linear autoencoder twin) + `anomaly.ts` | KEEP — PCA is the only deterministic autoencoder under sovereign-core |
| 9 | Neural ODE (business-flow dynamics) | Dynamics: order-rate trends | `eta.ts` (quantile ETA w/ intervals) + `governor.ts` PID/ICIR/resonance + `kalman.ts` | KEEP — continuous trend via Kalman/ETA, no ODE training |
| 2/3 | RNN/LSTM | (obsolete by Transformer) | none needed — `loop.ts` carries state explicitly | DROP (bloat, per directive) |
| 8 | DBN (2000s Bayes net) | (obsolete) | none needed — `kernel.ts` event-sourcing + `governor` Bayes-free meta-control | DROP (bloat, per directive) |

**Net**: keep 4/5/6/7/9 as roles; their *implementations* are already in the repo as deterministic
twins. Drop 2/3/8 as bloat. This confirms the previous principles analysis: bebop already has the
complexity-management stack — it is just expressed as pure math, not trained nets, which is exactly what
sovereign-core requires (no runtime RNG/SDG/Date).

## 2. Where is the real hole? (GNN-relation vs VAE-integrity)

The directive asks: is the hole in GNN (relational understanding) or VAE (anomaly detection)?

**Diagnosis — the hole is GNN / relational understanding, not VAE.**

- VAE-integrity already has TWO deterministic backstops: `cycle-consistency` (PCA round-trip, bounded
  error Σσ_j², proven blind spot) AND `anomaly.ts` (ICA/sparse-localization). The anomaly axis is the
  *most* covered part of the stack.
- GNN-relational understanding is the *weakest*: `arch-mine.ts` builds an adjacency and runs SVD to find
  coupling clusters, but it is a **static structure miner**, not a relational *reasoner*. It finds
  "these modules are coupled"; it does NOT answer "if order-node X changes priority, which delivery
  nodes bottleneck?" — that is exactly the GNN value the directive names. `dual-track.ts` is the closest
  (Truth Layer + `causalCounterfactual` blast-radius), but it reasons over code-deps, not order-graphs.
- So: **upgrade the relational axis.** Concretely: lift `causalCounterfactual` + `dual-track` from the
  code graph onto the order/role/delivery graph, and expose `bottleneckHint(focus)` so the orchestrator
  can re-prioritize (the directive's "Objective Function" tweak). This is the highest-EV gap and it is
  deterministic (BFS over the order adjacency — no GNN training).

## 3. Tensor / multipilot directive (brain-inside-brain)

The new standing directive: instead of 1–2 parallel verification loops, run **≥3 independent verifier
loops** and overlay their verdicts as a tensor (disagreement = a dimension of the artifact's risk). This
is the NEXT evolution of the "as-above-so-below" checker (Cross-pattern A) and it generalizes the
cycle-consistency blind-spot lesson ("one verifier can be fooled; N independent ones cannot all be").

Axes that compose into the overlay (each a DISTINCT method/model so they cannot collude):
- **Axis L — Logical/structural**: `logicalCot.verifyLogicalPlan` (preconditions/effects/invariants).
- **Axis A — Adversarial/red-team**: `redteam.redTeamProbe` (does the artifact survive mutation?).
- **Axis T — Truth/oracle**: `dualTrackGate` / contract oracle (does it contradict known facts?).
- (optional) **Axis I — Integrity**: `cycle-consistency` gap (does the artifact round-trip?).

`multipilot(artifact, loops[])` runs them in parallel, returns per-axis verdicts + an `overlay`:
`converged` (promote) or `divergent` (surface for human triage, never silently averaged). Default for
ALL agentic surfaces (reasoning/review/reverse-engineering/research/planning) — replaces single-checker
copilot as the default.

## 4. Recommendations (max-EV, applied in this commit)
- Promote principles 6/7/8 + Multipilot to AGENTS.md universal rules (done — this commit).
- Add `scripts/invariant-advisor-gate.mjs`: mechanically enforces "every advisor ⇒ deterministic
  verifier" (Cross-pattern B), so a future integration cannot add propose-and-execute.
- Add `src/integration/shadow.ts`: shadow-mode composition of logicalCot + dualTrack + validate over the
  loop's tool calls (non-blocking; observe FP rate before promoting any to a hard gate).
- Add `src/integration/multipilot.ts`: ≥3 independent verifier loops + tensor overlay (converged/
  divergent). Upgrade copilot → multipilot default.
- **Next**: lift `causalCounterfactual` onto the order graph (`bottleneckHint`) to close the GNN hole —
  the highest-EV remaining gap.

## 5. Evidence
- `arch-mine.ts`, `dual-track.ts`, `cycle-consistency.ts`, `anomaly.ts`, `kalman.ts`, `eta.ts`,
  `logicalCot.ts`, `redteam.ts`, `governor.ts` — all read this session; each is a deterministic twin of
  the corresponding "algorithm" above (no training).
- grep: flag_off=16, deterministic=123 — the stack is already air-gapped + pure.
