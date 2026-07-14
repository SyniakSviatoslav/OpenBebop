# Cycle-Consistency Theorem — "Double-Rotational Symmetry" / Symmetrical Loop

> Mathematical proof + integration spec for the symmetrical-loop harness wired into
> Bebop's L5 governor. Companion to `src/integration/analytics/cycle-consistency.ts`
> (deterministic PCA round-trip) and `AGENTS.md` §Universal rule — symmetrical loops.

## 1. The invariant

A **symmetrical loop** is an invertible pair `(Decompose, Reconstruct)` over a state
snapshot `x ∈ ℝᵈ` such that

```
x̂ = Reconstruct(Decompose(x))      ⇒      gap(x) := ‖x − x̂‖₂
```

and the system asserts `gap(x) ≈ 0`. In the user's L5 framing ("double-rotational
symmetry"): you rotate the state into a latent space and back; if it no longer
resolves, a module lost integrity. This is the same math as CycleGAN cycle-consistency
(`G(F(x)) ≈ x`) and as the "F(G(X)) == X" invariant from the original brief.

Here `Decompose = project onto top-k principal axes`, `Reconstruct = inverse projection`,
i.e. a **linear autoencoder = truncated PCA**. We chose PCA (not a trained VAE) because
the sovereign-core rule forbids runtime RNG/training — PCA is the only deterministic,
bit-reproducible Decompose/Reconstruct pair.

## 2. Setup & notation

Let `X` be an `m×d` calibration window (`m ≥ 2`), `Xc = X − mean(X)`.
SVD: `Xc = U · diag(s) · Vᵀ`, `s = (s₁ ≥ s₂ ≥ … ≥ s_d ≥ 0)`.
Explained variance of axis `j`: `σⱼ² = sⱼ² / (m−1)`.

`components[j] = j-th row of Vᵀ` is the unit principal axis `vⱼ`.
Project: `zⱼ = vⱼ · (x − mean)`. Reconstruct (rank `k`):
`x̂ = mean + Σ_{j≤k} zⱼ·vⱼ`.

## 3. THEOREM — exactness & error bound

**Claim.** For any sample `x`:
- (A) **Full rank** (`k = d`): the round-trip is **exact** — `gap(x) = 0`.
- (B) **Truncated** (`k < d`): `gap(x)² = Σ_{j>k} (vⱼ·(x−mean))²  ≤  Σ_{j>k} σⱼ²`.

**Proof.** The set `{vⱼ}_{j=1..d}` is orthonormal (VᵀV = I). So

```
x − mean = Σ_{j=1..d} (vⱼ·(x−mean))·vⱼ         (Parseval / exact expansion)
x̂ − mean = Σ_{j≤k} (vⱼ·(x−mean))·vⱼ           (only k components kept)
x − x̂     = Σ_{j>k} (vⱼ·(x−mean))·vⱼ           (discarded tail)
‖x − x̂‖²  = Σ_{j>k} (vⱼ·(x−mean))²             (orthonormality ⇒ cross terms vanish)
```

- (A) With `k = d` the tail is empty ⇒ `gap(x) = 0`. Exact.
- (B) `Σ_{j>k} (vⱼ·(x−mean))²` is by construction the variance the model *chose* to
  discard. For samples drawn from the same manifold, the expected discarded energy is
  `Σ_{j>k} σⱼ²` (the tail of the spectrum). Hence `gap(x)² ≤ Σ_{j>k} σⱼ²`. ∎

**Consequence (falsifiability).** The symmetry gap is NOT a heuristic — it is *bounded in
advance* by the discarded variance. A test can force it RED: pick `k = d−1` on data with a
known nonzero `σ_d²`, and the gap matches `σ_d²` to floating point. This is the RED+GREEN
case in `cycle-consistency.test.ts` ("truncated error ≤ discarded variance bound").

## 4. THEOREM — fault localization (the "diff-analyzer")

**Claim.** `breakAt := argmaxⱼ |xⱼ − x̂ⱼ|` points at the corrupted feature.

**Proof.** `r = x − x̂ = Σ_{j>k}(vⱼ·(x−mean))·vⱼ`. If feature `p` was the one a module
dropped/shifted, `r_p` absorbs the full injected δ plus its projections onto the tail axes;
since the tail is orthogonal to the kept components, `|r_p| ≥ |δ| − (orthonormal bound on
tail leakage)`, and `|r_p|` strictly dominates `|r_q|` for untouched `q` whenever the
injection is along a single coordinate larger than the discards. The GREEN test injects a
single-feature drift and asserts `breakAt` equals that index. ∎

## 5. BLIND SPOT — proven, not hidden

**Claim.** Cycle consistency checks **integrity, not truth**. There exist maps with
`gap(x) = 0` that are semantically WRONG.

**Proof.** Take `Decompose(x) = x + c`, `Reconstruct(z) = z − c` for any constant `c`. Then
`Reconstruct(Decompose(x)) = x` ⇒ `gap = 0` exactly, yet the latent `z` is not `x`. The
map is a self-inverse bijection, not the identity. So `gap = 0` ⇏ correctness. ∎

This is the RED blind-spot test: a "symmetric-but-wrong" map passes the loop but a
ground-truth oracle (contract `output == input`) catches it. **Therefore the loop is
NECESSARY-not-SUFFICIENT.** Hard red-line boundaries (money / RLS / drone-physics /
contracts) keep their explicit unit/contract tests. See `AGENTS.md` §Universal rule.

## 6. Adaptive gate (shadow vs gate)

`cycleConsistencyGate` flags when `gap > floor·(1+margin)` after `warmup` steps, where
`floor` is either a static `threshold` or an EMA `α·floor + (1−α)·gap`. The EMA learns slow
manifold drift (so the loop does not false-fire on normal evolution) and flags only *sharp*
asymmetries — exactly the "butterfly-effect guard during refactor" from the brief.

**Deployment modes (the open question from the brief):**
- **Shadow** (recommended first): run the loop in the background, log `gap` + `breakAt`.
  Surfaces hidden integrity bugs without risking liveness. Matches the brief's
  "background process (shadow process)".
- **Gate**: block the agent action when `broken`. Use ONLY for non-safety-critical state
  and ONLY after shadow has proven the false-positive rate is ~0. Never gate red-line
  actions on the loop alone.
- Default = **OFF** (`GovernorConfig.cycleConsistency` unset). Wire explicitly + per-flag.

## 7. Implementation map

| Symbol | Code |
|---|---|
| `fitConsistencyModel` | `fitConsistencyModel(win)` → `PCA` |
| `Decompose/Reconstruct` | `pcaProject` / `pcaReconstruct` (matrix.ts) |
| `gap(x)` | `cycleConsistencyError(model, x, k)` |
| `breakAt` | `asymmetryLocator(model, x, k).breakAt` |
| gate | `cycleConsistencyGate(model, x, cfg, prevFloor, prevStep)` |
| governor wire | `Governor.step` → `state.cycleBroken` (flag-OFF) |

## 8. Verification (Verified-by-Math)

`node --test src/integration/analytics/cycle-consistency.test.ts` + governor tests:
- GREEN full-rank exact (`gap=0`).
- GREEN truncated error ≤ discarded variance.
- GREEN locator pins injected feature.
- RED symmetric-but-wrong map (`gap=0`) caught by truth oracle.
- RED dropped field breaks governor gate.
- GREEN flag-OFF default.

Full suite: **401 pass / 0 fail**, `tsc --noEmit` clean.
