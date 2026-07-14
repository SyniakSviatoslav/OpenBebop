# Gortai analogy → bebop / bebop2 / dowiz

Date: 2026-07-13. Source: the long Gortai research dump framing generative-AI + adaptive
content systems as an electrical distribution network. Decision (operator): "all that was for
gortai — use it as an analogy for bebop, bebop2, dowiz".

Key finding from repo archaeology: bebop + bebop2 ALREADY implement the full electrotechnical /
cybernetic primitive set. The Gortai dump is NOT new math — it is a vocabulary that maps 1:1 onto
existing modules. The only genuinely missing piece was the *high-level closed-loop orchestrator*
that wires the math core into one controlled feedback loop. That gap is now filled by `resonator`
(see below).

## Mapping table (electrical ↔ cybernetic ↔ repo primitive)

| Electrical               | Cybernetic / genAI        | bebop (crates/bebop/src)        | bebop2 (bebop2/core/src)            | dowiz (agent-governance)        |
|--------------------------|---------------------------|----------------------------------|-------------------------------------|---------------------------------|
| Voltage (potential)      | Prompt quality / clarity  | `stabilizer` root-locus setpoint | `lyapunov::stability_margin`        | `index.ts` drift target         |
| Current (flow)           | Tokens/sec                | `wavefield` flux carrier         | `field` spectral propagator         | loop throughput metric          |
| Resistance (load)        | Compute budget            | `stabilizer` saturation limit    | `kalman` process noise `Q`          | resource ceiling (RSI-LOOP)     |
| Transformer (step-up)    | Context expansion         | `stabilizer` gain scheduling     | `chebyshev` propagator coeff        | context window scaling          |
| Transformer (step-down)  | Compression / distillation| `stabilizer` SMC sliding surface | `active` free-energy minimization   | summary / condensation          |
| Ground (reference)       | Immutable anchor / truth  | `stabilizer` ground_state        | `resonator::Reference`              | governance baseline             |
| Fuse (overcurrent)       | Max-iteration / budget cap| `stabilizer` saturation clamp     | `resonator` `LoopConfig.max_iter`   | RSI-LOOP iteration cap           |
| Capacitor (energy store)  | KV-cache / memory         | `wavefield` stored oscillation   | `vsa` hypervector bundle            | session memory                  |
| Oscillator (AC source)   | Sampling / temperature    | `stabilizer` oscillation mode    | `fft` frequency bin                 | temperature scheduler            |
| Closed loop (feedback)   | Self-correction           | `stabilizer` Lyapunov descent    | `active` active-inference          | `resonator` (NEW)               |
| Chaos watchdog           | Divergence freeze         | —                                | `resonator` lyapunov_guard (NEW)    | drift-accumulator (index.ts)    |
| Rollback                 | Reversion to best state   | —                                | `resonator::rollback_to_best` (NEW) | checkpoint/restore              |

## What already existed (do NOT rebuild)

- **bebop/crates/bebop/src/stabilizer.rs** — Lyapunov descent, sliding-mode control (SMC),
  root-locus setpoint scheduling, saturation clamp (the "fuse" + "transformer"), ground-state
  reference. This IS the electrical analogy in Rust, written first.
- **bebop/crates/bebop/src/wavefield.rs** — graph wave equation, spectral notch filter,
  capacitor-like energy storage. The "current/voltage wave" layer.
- **bebop2/core/src/lyapunov.rs** — eigen-decomposition + `stability_margin` / `is_stable` /
  `is_unstable` / `spectral_radius`. The quantitative stability probe.
- **bebop2/core/src/kalman.rs** — spectral covariance, process-noise `Q` (resistance/budget),
  trajectory integrals.
- **bebop2/core/src/active.rs** — active inference / free energy (the step-down transformer:
  minimize surprise).
- **bebop2/core/src/field.rs, chebyshev.rs, fft.rs, vsa.rs** — spectral propagators, FFT
  eigen-decomposition, vector-symbolic memory (capacitor).
- **dowiz/packages/agent-governance/index.ts** — governance with drift + error-patterns; the
  product-side port of the same cybernetics.

## What was ADDED (the gap)

### bebop2/core/src/resonator.rs  (host-gated, zero-dep, no_std-compatible core)
The high-level **closed-loop orchestrator** that the Gortai dump describes but the math core
lacked: an immutable `Reference` (ground), three pluggable actors — `Generator` (source/voltage),
`Reflector` (transformer + quality signal), `Supervisor` (circuit breaker) — driven around a loop
with:
- **Δ-threshold convergence** (error < ε → `Converged`),
- **max-iteration fuse** (`max_iterations` → `Fused`),
- **stall patience** (weak reflector, low quality plateau → `Stalled`),
- **drift accumulator** (sum of |step| — the "current that flowed the wrong way"),
- **Lyapunov chaos watchdog** (`lyapunov_guard`): if a step increases error (diverges), freeze
  adaptation and hold state — the overload protection,
- **rollback to best** (`rollback_to_best`): revert to lowest-error checkpoint.

Verified: `cargo test -p bebop2-core --features host resonator` → 6/6 green; full crate 166/166.

### dowiz port (planned, see RESONATOR-DESIGN.md)
`packages/agent-governance/resonator.ts` mirrors the same contract in TypeScript, plus RED+GREEN
tests in the style of `index.test.ts`. It reuses the existing `index.ts` drift accumulator rather
than duplicating it.

## Gortai (the product) — what it becomes
The Gortai concept (adaptive content for teenagers via a closed loop: Sensor → Comparator →
Controller → Actuator → Feedback) is now a *thin product layer* over `resonator`:
- Sensor = read learner state, Comparator = `metric` vs `Reference` (ground = age-appropriate
  baseline), Controller = `Generator`+`Reflector`, Actuator = content adaptation, Feedback =
  drift accumulator. The same loop that stabilizes a genAI output also stabilizes pedagogy.
No separate Gortai repo is needed — it is an application of `resonator` in dowiz.

## innovate: ceilings
- `resonator` uses a 1-D Lyapunov proxy inline (spectral_radius of a 2x2) for the watchdog to stay
  allocation-free per-tick. Upgrade trigger: wire `crate::lyapunov::spectral_radius` directly when
  the state `S` carries a square eigen-system (e.g. the deterministic math core's belief vector).
- `Metric` is user-supplied; the built-in `L2Metric` is the default. No convergence proof beyond
  "error decreases monotonically when the actor is contractive" — that is the caller's contract.
