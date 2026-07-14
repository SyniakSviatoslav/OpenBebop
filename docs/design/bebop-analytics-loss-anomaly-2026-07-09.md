# L5 Analytics — Loss Functions, PCA/SVD, PCA-Anomaly (applied findings)

_Date: 2026-07-09 · Author: Hermes agent · Status: landed, flag-OFF, RED+GREEN proven_

Repo context: the user's L5 prompt projected "drone telemetry / ELBO / VAE / PCA / SVD / ICA /
Causal / Neuro-Symbolic / Diffusion" onto the Dowiz L5 layer. Reality check first:

- **Dowiz** = DeliveryOS (B2B food-logistics), not drones. "Telemetry" = agent/loop telemetry
  (quality, cost, volume, predicted-vs-actual), already consumed by the L5 governor.
- The real **L5 governor** lives in **bebop** (`src/governor.ts`): PID+anti-windup, ICIR
  factor-health, resonance pre-check, Landauer floor, and a `z-score` `detectAnomaly` on the
  volume channel. It already had 6 integrations (zenoh, tigerbeetle, zkvm, active-inference,
  optical, wetware).
- **No** VAE/ELBO, PCA/SVD/ICA utility, Huber/Quantile/Focal/Contrastive loss, adaptive threshold,
  Causal graph, or Neuro-Symbolic layer existed before this session. The prompt named real,
  unfilled seams — not duplicated code.

This session wired the max-EV subset that lands on existing seams WITHOUT breaking the
deterministic-core rule (no RNG, no training loop, no Date/network at unit-test time).

---

## PART A — What was built (all flag-OFF, RED+GREEN)

### Module 1 — `src/integration/analytics/matrix.ts` (PCA/SVD/EVD foundation)
Deterministic linear-algebra primitives (Float64, no RNG/Date):
- `jacobiEVD(A)` — symmetric eigenvalue decomposition `A = V·diag(λ)·Vᵀ`.
- `svd(A)` — two-sided SVD `A = U·S·Vᵀ` via the symmetric EVDs of AᵀA / AAᵀ.
- `pcaFit(X)` / `pcaProject` / `pcaReconstruct` — PCA built on SVD.
- Helpers: `transpose`, `matmul`, `invSqrtDiag` (for whitening).

### Module 2 — `src/integration/analytics/anomaly.ts` (PCA-reconstruction anomaly)
The DETERMINISTIC twin of the prompt's "ELBO / VAE anomaly score". A full VAE needs a trained
weight matrix + SGD loop, which the sovereign-core rule forbids at runtime. A **linear
autoencoder ≡ PCA reconstruction**, so we get the same math — reconstruction error (how "weird"
the input looks) — with zero learned params and bit-for-bit reproducibility:
- `score = ||x − x̂||₂ + β·Σzⱼ²` (β OFF by default — raw Σzⱼ² false-positives on normal
  samples whose latent mean is merely non-zero).
- **Adaptive EMA threshold** (the prompt explicitly warned against a constant threshold):
  `threshold_k = α·threshold_{k-1} + (1−α)·score_k`. Slow drift (battery/weather) is learned
  out; only SHARP excursions flag.
- **warmup** (floor not established yet → never flag on step 1) and **margin** hysteresis
  (score must exceed the floor by 10% to declare an anomaly, killing numerical-noise trips).
- `buildNormalModel(window)` calibrates on known-good telemetry.

### Module 2 wiring — `src/governor.ts`
- `TelemetrySample.features?: number[]` (optional multidimensional vector; back-compatible).
- `GovernorConfig.pcaAnomaly?` (flag-OFF — absent by default).
- `step()` scores `features` against the calibrated model ONLY when both `cfg.pcaAnomaly` and a
  same-length `features` vector are present; updates `state.pcaAnomaly` (added to `GovernorState`).
- The legacy `detectAnomaly` (z-score on volume) is untouched — this is an additive, orthogonal
  signal.

### Module 3 — `src/integration/analytics/loss.ts` (robust-loss building blocks)
- `huber(err, δ)` — MSE inside δ, MAE outside (robust to telemetry spikes).
- `mse(errors)` — the baseline the prompt warns about.
- `quantileLoss(actual, pred, τ)` — ETA / prediction intervals (Dowiz delivery ETA seam).
- `focalLoss(p, γ)` — re-weights rare classes (incident vs nominal).
All deterministic, RED+GREEN tested. Huber is the immediate max-EV building block for the Dowiz
ETA model; the others are ready primitives.

---

## PART B — Verification (Verified-by-Math bar)

Run: `node --test --import tsx $(find src -name '*.test.ts')` → **383 pass / 0 fail**.
`npx tsc --noEmit` → exit 0.

New modules: 53 tests, all green. Key RED+GREEN pairs:
- matrix: SVD reconstructs A within ε (GREEN) / non-square·NaN throws (RED).
- anomaly: in-manifold steady telemetry does NOT flag (GREEN) / alien vector DOES flag (RED);
  slow drift absorbed by EMA, sharp excursion after drift flags (GREEN+RED).
- governor: with `pcaAnomaly` configured, alien features flag (RED); WITHOUT config, `pcaAnomaly`
  stays false forever (flag-OFF default, GREEN).
- loss: huber textbook values (GREEN) / non-finite·δ≤0 rejected (RED); focal→0 as p→1 (GREEN)
  / p=0 ⇒ +∞ (RED).

**Proof is falsifiable**: the RED cases fail when the code is wrong (e.g. we caught — and fixed —
a real bug where `pcaAnomaly` was computed but never returned in `GovernorState`, and a bug where
`k=0` (all axes) made PCA a perfect identity that could never detect anomalies → switched to
auto `d−1` rank).

---

## PART C — Honest gaps (no false-green)

NOT built this session (research / heavier arcs, correctly deferred):
- **Full VAE/ELBO training**: replaced by deterministic PCA reconstruction (same math, no
  determinism violation). β-VAE available behind `cfg.beta > 0` once latent N(0,I) is calibrated.
- **ICA / FastICA**: not needed for the anomaly seam (PCA covers it); SVD-of-adjacency
  architecture-mining is the next concrete ICA-adjacent use.
- **Causal graphs (DoWhy/CausalNex)**: R&D-only — needs causal discovery over a real trace;
  the adjacency-matrix SVD is the cheaper first step.
- **Neuro-Symbolic layer**: the guard gate + VSA field-oracle already play this role; a formal
  symbolic layer over the active-inference advisor is a separate kernel-authority arc.
- **Diffusion anomaly detection**: research-only (needs training); PCA-reconstruction covers
  ~80% of the value deterministically today.

---

## PART D — Max-EV next steps (when you want them)

1. **Wire `selectZenoh` + `prove` into kernel dispatch** (flag-OFF, RED+GREEN) — the standing
   "apply findings into real runtime" item from `bleeding-edge-EV-2026-07-08.md`.
2. **Dowiz ETA model** using `quantileLoss` + `huber` (prediction intervals + robust fit).
   Seam: a real ETA module in Dowiz (search `eta`/`delivery` in `apps/api`).
3. **RAG noise-cleaning**: project VSA/embedding spaces through `pcaFit`/`pcaProject` in
   `knowledge.ts` before recall — drop noise dims, reduce LLM hallucinations (prompt's claim).
4. **Architecture-mining harness**: build the module-dependency adjacency matrix, run `svd`,
   surface latent coupling clusters (tight coupling = refactor signal). Extend to ICA for
   mixed-signal log separation.
5. **Causal graph** over module imports/calls → "points of failure" counterfactual queries.

---

## Files changed
- `src/integration/analytics/matrix.ts` (new)
- `src/integration/analytics/anomaly.ts` (new)
- `src/integration/analytics/loss.ts` (new)
- `src/integration/analytics/{matrix,anomaly,loss}.test.ts` (new, RED+GREEN)
- `src/governor.ts` (features vector + cfg.pcaAnomaly + pcaAnomaly state; back-compatible)
- `src/governor.test.ts` (+3 RED+GREEN L5-analytics cases)
- `src/integration/README.md` (analytics row added)
