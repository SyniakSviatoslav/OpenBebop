# bebop L5 — Applied research synthesis + max-EV roadmap

_Date: 2026-07-09 · Author: Hermes agent · Status: RESEARCH + PLAN (no new code in this doc; prior wave D0–D6 already landed & verified)_

This is the synthesis of a large multi-topic research dump (loss functions, ELBO/VAE,
PCA/SVD/ICA blind spots, Causal AI, Neuro-Symbolic, Diffusion, RLHF/PPO/World-Model,
QCD/Lagrangian/C-Space/RRT*, Sandbox Paradox, Controller-Observer, tensor covariance,
HDC/Geometric Algebra, quantum critique, Graph-vs-Tensor memory, GNN hybrid, ADR) into a
**concrete, philosophy-consistent plan** for the bebop L5 layer + the Dowiz sovereign node.

Ground-truth lens (from HERMES.md / bleeding-edge-EV-2026-07-08.md / the two prior 2026-07-09 plans):
- **Deterministic Rust/WASM core** — event-sourced, zero dynamic alloc at the money boundary,
  `decide()` invents no money number, replayable bit-for-bit.
- **Sovereign / offline-first** — air-gapped, local LLM (Ollama), no OpenAI/Anthropic egress.
- **Verified-by-Math** — every change needs a falsifiable RED+GREEN proof. No false-greens.
- **Anti-hallucination by architecture** — a *deterministic governor* gates the *stochastic agent*.
  The agent advises; the kernel executes. (Controller-Observer split, already the bebop shape.)
- **Max-EV sequencing** — Zenoh (now) > RISC Zero zkVM (money boundary) > TigerBeetle (reference)
  > pymdp/RxInfer (design-only) > FinalSpark (ethics/excluded).

---

## 0. Ground truth — what ALREADY exists (do not rebuild)

The 2026-07-09 D0–D6 wave landed all of this in `bebop` (flag-OFF, RED+GREEN proven; `npm run verify` GREEN at 434 pass / 0 fail):

| Module | File | What it already covers from the dump |
|---|---|---|
| Linear-algebra foundation | `src/integration/analytics/matrix.ts` | SVD / PCA / EVD — the "SVD/PCA" the dump asks for |
| PCA-reconstruction anomaly | `src/integration/analytics/anomaly.ts` | The deterministic twin of the dump's "ELBO/VAE anomaly score" (reconstruction term + β·Σzⱼ² latent-KL, β OFF by default, **adaptive EMA threshold** the dump explicitly demanded) |
| Robust losses | `src/integration/analytics/loss.ts` | Huber, MSE, Quantile, Focal — the dump's "Huber Loss" + ETA interval primitive |
| Symmetry-loop theorem | `src/integration/analytics/cycle-consistency.ts` | F(G(X))==X — the "Sandbox Paradox" counter-measure (currently HARD equality) |
| ICA telemetry | `src/integration/analytics/ica.ts` + `telemetry-ica-loop.ts` | Sparse-source localization; RED case = Gaussian blind-spot |
| Telemetry shadow | `src/integration/analytics/telemetry-shadow.ts` | Calibrated ICA + structural-drift detector, report-only |
| ETA intervals | `src/integration/analytics/eta.ts` | `quantileLoss`+`huber` interval forecaster (Dowiz ETA seam) |
| Architecture mining | `src/integration/analytics/arch-mine.ts` | SVD-of-adjacency over module imports → coupling clusters/cycles (the dump's "Causal graph" *first* step) |
| Governor (the gate) | `src/governor.ts` | PID + ICIR + resonance pre-check + Landauer floor + `detectAnomaly` (z-score) + flag-OFF `pcaAnomaly` + `cycleBroken` + `subsystemFault` |

**Conclusion:** the dump's "Huber Loss", "KL/Anomaly/ELBO", "PCA/SVD", "adaptive threshold",
"symmetry loop", "causal graph first step" are *already built*. Rebuilding them would be
anti-EV. The plan below is about (a) the *refinements* those modules still need, and
(b) the items the dump raised that are genuinely new.

---

## 1. The dump, decoded into four classes

### Class A — ALREADY BUILT (keep, do not rebuild)
Huber, MSE, Quantile, Focal · PCA reconstruction anomaly · adaptive EMA threshold ·
β-latent-KL (behind `cfg.beta`) · SVD/PCA/EVD · symmetry-loop theorem · ICA localization ·
architecture-mining. → Nothing to do except calibrate/extend (see N3, N4).

### Class B — MAX-EV, philosophy-consistent, lands on existing seams (NEXT WAVE)
These are real, buildable, and violate no core rule (no training/RNG at runtime; offline
training only where a model is needed, edge-inference only):
- **N1 Open-System Symmetry** — relax the hard `F(G(X))==X` to a *tolerance band*
  `||F(G(X))−X|| ≤ tol` + an entropy-injection test (the dump's "Sandbox Paradox" fix).
  Directly upgrades `cycle-consistency.ts`. ★ highest EV.
- **N2 Governor liveness contract** — heartbeat/watchdog/safe-state: if the stochastic
  agent goes silent past `X` ms, kernel drops to Safe State. The dump's checklist item 3.
  ★ high EV, missing today.
- **N3 β-VAE latent-KL calibration harness (offline)** — calibrate the N(0,I) prior against
  normal telemetry, then flip `cfg.beta>0`; RED+GREEN proves on/off behavior.
- **N4 Causal counterfactual surface** — extend `arch-mine` D6 from "find cycles/orphans"
  to an explicit "points of failure" query (the dump's Causal-Graph ask, cheap-first).
- **N5 Neuro-Symbolic gate ADR** — formalize the Governor + VSA field-oracle as the
  *symbolic layer* over the stochastic advisor. Already plays this role; document as ADR.
- **N6 Dual-Track GNN hybrid (design + offline seam)** — Truth = petgraph/deterministic
  governor; Operational = tensor analytics. Training (if any) in PyG/DGL **offline**,
  export SafeTensor, infer in Candle/Burn on edge. Flag: **training never runs in core**.
- **N7 Hybrid-bridge observability** — the dump's "are you degrading 10 min before failure?"
  metric: expose hallucination-rate (governor-rejected advices), GNN/analytics latency,
  compute budget, as a telemetry surface.

### Class C — REJECT / DEFER (with reasons — not silently dropped)
| Item from dump | Verdict | Why |
|---|---|---|
| VAE / ELBO **training** (SGD + RNG) | DEFER (offline only) | Sovereign-core rule forbids training loops at runtime; the deterministic PCA twin already covers anomaly. Train offline, infer edge-only if ever. |
| Diffusion anomaly detection | DEFER (research) | Needs a trained model; PCA-reconstruction covers ~80% of value deterministically today. |
| PPO / RLHF / RLAIF / Dreamer World-Model | DEFER (design-only) | Needs an env + training loop. The governor's ICIR/PID/resonance feedback IS the deterministic "self-correction" meta-layer; PPO is a metaphor, not a build. |
| Quantum computing | REJECT | Dump itself calls it a trap; physically incompatible with autonomous edge (decoherence, cryo, latency). |
| Geometric Algebra | DEFER | Math-perfect but x86/ARM can't execute natively; 90% emulation cost kills L5 latency. Research only. |
| HDC (Hyperdimensional Computing) | DEFER (research) | Real "Semantic Drift" blind spot: noise accumulates → code-word loses entity link while math stays "valid". Experimental. |
| DoWhy / CausalNex (causal libs) | DEFER (offline design) | Python; cannot run in core. Use offline to validate the `arch-mine` counterfactual surface. |
| FEP / Active Inference (pymdp/RxInfer) | PARTIAL (design-only) | Already the *design language*; reimplement policy in Rust (per bleeding-edge-EV). Governor resonance-pre-check ≈ "predict consequences before acting". |

### Class D — PRINCIPLES TO KEEP (the dump's good advice, already consistent)
- **Controller-Observer / Data-Plane vs Control-Plane** — already the bebop shape; encode as ADR.
- **Open-System Symmetry** (relaxed equality + entropy injection) — see N1.
- **Safe State / Heartbeat / Watchdog** — see N2.
- **Type-safe coordinate frames** (`PhantomData<Frame>`) — belongs to the **Rust core (dowiz-core)**, not bebop TS. Note for phase-zero.
- **Observability-centric > code-centric** — see N7.
- **Failure-mode testing** (agent garbage / sensor noise / OOM → safe state) — add RED tests.
- **ADR discipline** — write the hybrid/governor ADR (N5).

### Category-error callout (honest)
The dump repeatedly frames Dowiz as a *drone* (C-Space, RRT*, QCD Lagrangian, rotors,
wind). **Dowiz = DeliveryOS (B2B food-logistics), not drones.** Keep the *principles*
(open-system symmetry, controller-observer, safe-state, type-safe frames) and drop the
*literal* rotor/RRT*/Lagrangian implementation. The "telemetry" the dump targets = agent/loop
telemetry (quality, cost, volume, predicted-vs-actual) already consumed by the governor.

---

## 2. Next-wave plan (the buildable, philosophy-consistent items)

Each item: seam · flag-OFF · RED+GREEN proof requirement · doc-claim gate extension (added
ONLY when the item lands, so the gate stays green).

### N1 — Open-System Symmetry (upgrade `cycle-consistency.ts`)
- **Seam:** `cycle-consistency.ts` currently asserts hard `F(G(X))==X`. Add `cfg.symmetryTol`
  (default 0 = legacy exact; >0 = tolerance band) and a `entropyInjection` test vector.
- **Why max-EV:** directly answers the dump's "Sandbox Paradox" — a hard equality is brittle
  in a noisy real world; a tolerance band + entropy injection makes the check robust without
  weakening the guarantee.
- **RED+GREEN:** GREEN — in-manifold sample within tol passes; RED — corrupted sample outside
  tol fails; RED — entropy-injected sample still resolves correctly (robustness proof).
- **Flag-OFF:** `symmetryTol` absent ⇒ exact-equality legacy behavior preserved.

### N2 — Governor liveness contract (heartbeat / watchdog / safe-state)
- **Seam:** new `GovernorState` fields `agentSilentMs`, `safeState` + a `step()` branch:
  if `now − lastAgentMsg > cfg.watchdogMs` ⇒ `safeState=true`, authority→0.
- **Why max-EV:** the dump's checklist item 3; a stochastic agent that "hangs thinking" must
  not hold the wheel. Closes a real safety gap.
- **RED+GREEN:** GREEN — agent responsive ⇒ safeState false; RED — silence > watchdogMs ⇒
  safeState true + authority clamped (no bypass).
- **Flag-OFF:** `cfg.watchdogMs` absent ⇒ liveness check disabled (legacy).

### N3 — β-VAE latent-KL calibration (offline, flip `cfg.beta`)
- **Seam:** `anomaly.ts` already has `beta`; add `calibrateLatentPrior(window)` that fits N(0,I)
  and asserts latent mean≈0/var≈1 on normal data; flip `beta>0` only after calibration.
- **RED+GREEN:** GREEN — calibrated β improves separation on sharp excursion; RED — uncalibrated
  β false-positives on normal non-zero-mean latent (the doc already warns about this).

### N4 — Causal counterfactual surface (extend `arch-mine.ts` D6)
- **Seam:** add `pointsOfFailure(graph)` returning the cycle/orphan set as an explicit
  counterfactual query ("if module X changes, what breaks?").
- **RED+GREEN:** GREEN — known cycle reported; RED — broken edge NOT silently absorbed.

### N5 — Neuro-Symbolic gate ADR
- **Seam:** doc only (ADR) + a one-line cross-link from `AGENTS.md` to the Governor-as-gate
  pattern. No runtime change yet.
- **Why:** the dump's strongest, already-consistent architectural claim; recording the
  decision (what/why/rejected) is the "architect" move the dump asks for.

### N6 — Dual-Track GNN hybrid (design + offline seam)
- **Seam:** design doc + a typed seam (`TruthLayer` trait / `TensorLayer` interface) so a
  future trained model can be dropped in without touching the governor. Training offline
  (PyG/DGL) → SafeTensor → Candle/Burn edge inference. **Training never in core.**
- **Flag:** `cfg.gnnInference` absent ⇒ tensor layer = current deterministic analytics only.

### N7 — Hybrid-bridge observability
- **Seam:** extend `GovernorState` / a new `bridgeMetrics()` with `hallucinationRate`
  (rejected advices / total), `analyticsLatencyMs`, `computeBudgetUsed`.
- **RED+GREEN:** GREEN — metrics emitted under load; RED — a rejected advice is NOT counted
  (proves the counter is honest).
- **Why:** the dump's "architect" test — "how will the system tell me it's degrading 10 min
  before it fails?" This is the telemetry that answers it.

---

## 3. Sequencing (operator's "apply findings into real runtime FIRST, then plan")

Per the operator's standing workflow, the highest-EV buildable items must be **wired into the
real runtime (flag-OFF, RED+GREEN) before the next elaborate plan**. So:

1. **N1 + N2 first** (Open-System Symmetry + liveness contract) — both land on existing seams,
   both close real gaps the dump identified, both falsifiable. Implement, prove, wire.

---

## 6. N1 + N2 — IMPLEMENTED & VERIFIED (2026-07-09)

Both items were wired into the real runtime (flag-OFF, RED+GREEN) and the doc-claim gate
was extended. Final proof:

- `npm run verify` → **441 pass / 0 fail** (was 434 before this work; +7 = 3 N1 + 4 N2).
- `.git/hooks/pre-commit` → both gates GREEN (doc-claim checks M + N added; falsifiable-proof 55/55).
- README.md / AGENTS.md test counts updated to 441 (the doc-claim gate caught the drift and it was fixed).

### N1 — Open-System Symmetry (`cycle-consistency.ts`)
- `CycleConsistencyConfig.symmetryTol` (optional, default 0 = legacy exact F(G(X))==X).
- Breach = gap exceeds `max(floor·(1+margin), prevThreshold + tol)`: the EMA floor still
  absorbs slow DRIFT, but a SHARP jump past the established baseline by more than `tol`
  breaks. This is the Sandbox-Paradox fix — stochastic world tolerated, abrupt break flagged.
- Tests: GREEN (small z-axis drift 0.05 within band → not broken) · RED (dropped field 0.5 gap
  breaks even with band) · RED (tol=0 recovers the brittle exact-equality breach).

### N2 — Liveness contract / safe-state (`governor.ts`)
- `GovernorConfig.watchdogMs` (optional, flag-OFF) + `step(s, nowMs?)` (4th arg, optional →
  all 434 prior call sites stay green).
- Each clocked `step` is a heartbeat; if the gap between advisories exceeds `watchdogMs`, the
  kernel drops to **Safe State** (`safeState=true`, `authority` floored to `uMin`). Watchdog is
  inert when no clock is ever supplied (cannot false-trip on a missing clock).
- `GovernorState` gains `safeState?` + `agentSilentMs?`.
- Tests: GREEN (responsive heartbeat never trips) · RED (silence past budget → Safe State +
  authority floored) · GREEN (no-clock caller never trips) · GREEN (no `watchdogMs` config never trips).

### Not done yet (next in sequence, parked until you say otherwise)
N3 (β-VAE prior calibration) · N4 (causal counterfactual surface) · N5 (Neuro-Symbolic ADR) ·
N6 (GNN seam) · N7 (observability). Each is flag-OFF / design-only per the philosophy rules;
no training loop or RNG in the core.

Deferred (Class C) stay frozen; re-open only when an offline-trained model is actually needed
and the air-gap/determinism constraints can be satisfied (edge-inference only).

---

## 4. Doc-claim gate extensions (add when each item lands, keep gate green)

- **M (N1):** `cycle-consistency.ts` exposes `symmetryTol` cfg AND a tolerance-band test exists
  (GREEN within tol / RED outside). RED case: delete tol → exact-equality-only test fails.
- **N (N2):** `governor.ts` exposes `safeState` + `agentSilentMs` AND `step()` clamps authority
  on silence. RED case: remove the clamp → test fails.
- **O (N3):** `anomaly.ts` `calibrateLatentPrior` exists + test asserts uncalibrated-β
  false-positive (RED) and calibrated-β separation (GREEN).
- **P (N4):** `arch-mine.ts` `pointsOfFailure` exists + test asserts a known cycle is reported.
- **Q (N7):** governor exposes `hallucinationRate` + test asserts a rejected advice is counted.

(Do NOT add M–Q until the code lands — adding a check for absent code would RED the gate.)

---

## 5. Honest gaps in THIS plan
- No code written here — this is the analysis + roadmap the operator asked for. The D0–D6 wave
  (prior plan) is the implemented baseline; this plan's N1–N7 are the next wave.
- The drone-physics framing in the source dump is treated as a category error (Dowiz ≠ drones);
  only the transferable principles are kept.
- Class C deferrals (VAE-training, Diffusion, PPO/Dreamer, Geometric Algebra, HDC, causal libs)
  are explicitly rejected/deferred *with reasons* so they are not silently revived.
- Current test count is the doc-claim gate's live source of truth (last committed run = 434
  pass / 0 fail); do not hardcode — re-derive via `npm run verify`.
