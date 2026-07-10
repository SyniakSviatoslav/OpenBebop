# active-inference — reverse-engineering notes

## Sources (reverse-engineered)
- **pymdp** (Python POMDP Active Inference) — `infer-actively/pymdp` (PyPI `pymdp` is an
  unrelated name-squat; the real package installs as `infer-actively-pymdp`). Captured REAL
  stdout from a 2-state/2-action model (see "Ground truth" below).
- **RxInfer.jl** (Julia reactive message passing) — live source/doc fetch was blocked (401 / 404),
  so the documented decomposition `G = risk − ambiguity + info_gain` was used; it matches the
  pymdp form, which we verified independently.

## Exact FEP equations (verified against pymdp)
Belief update (single factor, FPI with EPS in all logs):
```
qs = softmax( log A[:,obs] + log D )           # infer_states
```
Expected Free Energy (pymdp reports it as **negative EFE, G — larger is BETTER**):
```
for each step t, action a_t:
  qs_{t+1} = B[:,:,a_t] · qs_t                  # advance belief by the action FIRST
  qo       = A · qs_{t+1}                        # predicted obs from POST-action belief
  utility  = Σ_o qo[o] · log( softmax(C)[o] )   # C is a LOG-DOMAIN preference (may be negative)
  infoGain = Σ_s qs[s]·H[A[:,s]] − H[qo]         # ambiguity − observation entropy
  G       += utility + infoGain
policy = argmax_p G                             # selectPolicy maximizes G
```
Key corrections made during integration (caught by grounding to REAL pymdp output):
1. `C` is a **log-domain preference vector** (e.g. [-2, 0]), NOT a probability distribution.
   Using `KL(predObs, C)` hit `log(negative)` → NaN. Correct utility is `Σ qo·log(softmax C)`.
2. pymdp **maximizes** G (negative EFE), not minimizes. `selectPolicy` uses `argmax`.
3. Observation is predicted from the **post-action** belief (action moves state, then you observe),
   not the pre-action belief. Reordering the loop was required to match pymdp's G exactly.

## Ground truth (pinned in ai.test.ts)
Model: `A=[[.95,.05],[.05,.95]]`, `B[:,:,0]=I`, `B[:,:,1]`=flip, `C=[-2,0]`, `D=[1,0]`, obs=1,
policy_len=1 →
```
POSTERIOR q(s|o) = [1.  0.]
NEG_EFE G        = [-2.026928  -0.226928]
CHOSEN ACTION     = [1.]
```
`inferStates` and `selectPolicy` reproduce these to tolerance <1e-3. This is the falsifiable
anchor: the module is proven against REAL pymdp, not just internal consistency.

## Determinism / limits
- Pure Float64 math, no RNG, no `Date`. Same input → same output (RED test: determinism).
- `NaN`/non-finite preference or prior throws (RED test) — no silent NaN policy.
- Policy enumeration is `actions^horizon`; fine for small horizons (the Sovereign Node loop
  uses short horizons; for large spaces a beam search would replace brute force).
