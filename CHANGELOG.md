# Changelog

All notable changes to Bebop are documented here. Format: [Keep a Changelog](https://keepachangelog.com/),
versions are semver, every entry is backed by a falsifiable test (RED+GREEN) run in CI.

## [0.4.0] — 2026-07-09 (c)

Codename: **"Multipilot + the new outfit."** Field core, planner, and identity work.

### Added
- **Multipilot** (`src/copilot.ts::runMultiPilot`): a task fans out to N *specialist* pilots (distinct
  backends so no single failure mode dominates), a *distinct* synthesizer merges them, and the Rust
  field arbiter (`rustFieldArbiter`) can veto the plan. Exposed as `bebop multipilot "<task>"`.
  RED+GREEN: distinct-pilot invariant, field override blocks, single-roster fallback.
- **New outfit** (`src/outfit.ts`): the cosmo-noir identity as one source of truth — Warm Cosmo-Noir
  (Cowboy Bebop × cosmo-gothic × Ukrainian irony), signal teal `#46B0A4`, bone `#F2E9DB` on void
  `#12100E`, creed "Hybrid is a feature, not a bug." `bebop outfit` prints it. Versioned v1.0.0.
- **Sensitivity bootstrap (zero new infra)**: the kernel accrues `|Δu|` per step into `ACCUM`;
  `field_sensitivity()` returns the per-node energy (most-active node = 1.0). `rustFieldSensitivity()`
  auto-bootstraps when the planner doesn't pass one. Verified GREEN: source node > quiescent tail.
- **f32-packed CSR** (`field_build_f32`): CSR col-indices uploaded as f32 (halves storage); compute stays
  f64 — results bit-identical to f64 CSR (max diff < 1e-12). Lifts the prior `n≈2000` binding limit.
- **SIMD128** (`-C target-feature=+simd128` in `rust-core/.cargo/config.toml`): measured **1.08×** faster
  (n=1500, 300 iters). Modest but free, stable, deterministic. We report the measured number, not a claim.
- **Top-K Contours** (`rustTopKContours` + `field-planner.ts::fieldGatePlan`): the planner seam now
  annotates every action with `{verdict, fieldCost, contours}` — the explainability surface that makes
  "the arbiter overrode PDDL" auditable.
- **Field-sim comparison report + visual explainer** (`scripts/field-sim-report.mjs` →
  `docs/design/field-sim-comparison-2026-07-09.md` + `docs/diagrams/field-sim-explainer.svg`): the
  **unique feature** — Bebop's planner reads a *deterministic graph-PDE field* as its cost function, and
  you can *see* where a disruption will hurt.

### Changed
- wasm `--max-memory` ceiling lifted 64 MiB → **1 GiB** (larger graphs fit).
- `rustFieldArbiter` is now a sensitivity-aware wrapper over `rustFieldArbiterCore`.

### Verified
- Rust kernel: **16** tests (`cargo test -p bebop-core`), wasm32 build clean.
- TS suite: **547** tests (`npm test`), 0 fail.
- `npm run typecheck`: 0 errors.
- `node scripts/verify-doc-claims.mjs`: all doc claims backed by live proof.

## [0.3.5] — prior
- Rust→WASM field core (Chebyshev spectral + active-set), PDDL↔field bridge (Final Arbiter).
- Optical search + real-time change prediction (`predictImpact`/`opticalNodeSearch`/`vsaNodeSearch`).

## [0.3.0] — prior
- zkVM `decide()` journal (keyless tamper-*detection* digest; not a cryptographic MAC — see note F5).
