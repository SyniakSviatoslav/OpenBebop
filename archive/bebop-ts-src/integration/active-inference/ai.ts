/**
 * ai.ts — Deterministic Active Inference (Free-Energy Principle) policy selector.
 *
 * Reverse-engineered from:
 *   - pymdp (Python POMDP Active Inference): `infer_states` (precision-weighted PE
 *     minimization via softmax of -precision·prediction_error) and `infer_policies`
 *     (Expected Free Energy = risk + ambiguity, select argmin-EFE policy).
 *   - RxInfer.jl (reactive message passing): the same belief update expressed as
 *     Bayes/minimum-divergence; EFE = E[ln q(o)] - E[ln p(o|s,π)] - H[q(a)].
 *
 * The core law (shared by both): an agent minimizes a variational FREE ENERGY.
 *   F = risk (divergence of predicted obs from preferred obs)
 *     + ambiguity (expected uncertainty of obs under the policy)
 *     + novelty (epistemic value, optional)
 * The chosen policy is the one minimizing Expected Free Energy.
 *
 * Everything is DETERMINISTIC: Float64 categorical distributions, no RNG, no Date,
 * no IO. The math is the same whether the engine is Python (pymdp), Julia (RxInfer)
 * or Rust/WASM — only the solver differs. This is the "compute the belief" primitive.
 */
export type Dist = number[]; // categorical prob vector, sums to 1
export type Matrix = number[][]; // rows = from, cols = to (transition/likelihood)

export function softmax(x: number[]): Dist {
  for (const v of x) if (!Number.isFinite(v)) throw new Error('softmax: non-finite input');
  const m = Math.max(...x);
  const exps = x.map((v) => Math.exp(v - m));
  const s = exps.reduce((a, b) => a + b, 0);
  return exps.map((e) => e / s);
}

export function kl(a: Dist, b: Dist): number {
  let r = 0;
  for (let i = 0; i < a.length; i++) {
    if (a[i] > 0) r += a[i] * Math.log(a[i] / (b[i] + 1e-12));
  }
  return r;
}

export function entropy(d: Dist): number {
  let r = 0;
  for (const p of d) if (p > 0) r -= p * Math.log(p);
  return r;
}

/** precision-weighted prediction-error belief update (pymdp infer_states). */
export function inferStates(
  prior: Dist,
  likelihood: Dist, // p(o|s) for the observed outcome
  precision = 1,
): Dist {
  // Variational update (minimize variational free energy): ln q(s) ∝ ln prior(s) + precision·ln p(o|s).
  // (Equivalent to precision-weighted PE minimization: the posterior is sharpened toward the
  // likelihood as precision rises.)
  const logPost = prior.map((p, i) => Math.log(p + 1e-12) + precision * Math.log(likelihood[i] + 1e-12));
  return softmax(logPost);
}

export interface PomdpModel {
  /** A: observation likelihood A[o][s] = p(o|s). */
  A: Matrix;
  /** B: transition B[s'][s,a] = p(s'|s,a). Indexed [action][to][from] (3D). */
  B: number[][][];
  /** C: log-domain preferences over observations C[o] (pymdp preference; may be negative). */
  C: number[];
  /** D: prior over initial states D[s]. */
  D: Dist;
  /** numActions. */
  actions: number;
}

/**
 * Expected Free Energy of a policy (pymdp "negative EFE", G — larger is BETTER).
 * G = Σ_t [ utility_t + infoGain_t ]
 *   utility_t   = Σ_o q(o)·log(softmax(C)[o])          (risk: reach preferred obs)
 *   infoGain_t  = Σ_s q(s)·H[A(:,s)] − H[q(o)]        (ambiguity − obs entropy)
 * C is a log-domain preference (matches pymdp C, which can be negative).
 */
export function expectedFreeEnergy(
  model: PomdpModel,
  policy: number[],
  precision = 1,
): number {
  const { A, B, C, D } = model;
  if (!Number.isFinite(precision) || precision <= 0) throw new Error('expectedFreeEnergy: precision must be finite > 0');
  for (const c of C) if (!Number.isFinite(c)) throw new Error('expectedFreeEnergy: non-finite preference C');
  for (const d of D) if (!Number.isFinite(d)) throw new Error('expectedFreeEnergy: non-finite prior D');
  const nS = D.length;
  const nO = A.length;
  let qs: Dist = D.slice();
  let g = 0; // pymdp "negative expected free energy" — larger is BETTER
  for (let t = 0; t < policy.length; t++) {
    const a = policy[t];
    // advance belief FIRST (action a_t moves state), then predict the resulting observation
    const Bb = B[a];
    const next: Dist = new Array(nS).fill(0);
    for (let sp = 0; sp < nS; sp++) {
      for (let s = 0; s < nS; s++) next[sp] += qs[s] * Bb[sp][s];
    }
    qs = next;
    // predicted observation from the post-action belief: q(o) = Σ_s q(s) · A[o][s]
    const predObs: Dist = new Array(nO).fill(0);
    for (let o = 0; o < nO; o++) {
      for (let s = 0; s < nS; s++) predObs[o] += qs[s] * A[o][s];
    }
    // utility (risk): Σ_o q(o) · log(softmax(C)[o])  — C is log-domain preference (may be <0)
    const pref = softmax(C);
    let u = 0;
    for (let o = 0; o < nO; o++) u += predObs[o] * Math.log(pref[o] + 1e-12);
    g += u;
    // info gain (ambiguity − observation entropy): Σ_s q(s)·H[A(:,s)] − H[q(o)]
    let ig = 0;
    for (let s = 0; s < nS; s++) {
      const col: Dist = new Array(nO);
      for (let o = 0; o < nO; o++) col[o] = A[o][s];
      ig += qs[s] * entropy(col);
    }
    ig -= entropy(predObs);
    g += ig;
  }
  return g; // larger = better (pymdp "negative EFE")
}

/** Select the policy minimizing EFE (pymdp infer_policies). */
export function selectPolicy(model: PomdpModel, horizon: number, precision = 1): {
  policy: number[];
  efe: number[];
} {
  const { actions } = model;
  // enumerate all policies (actions^horizon); deterministic, bounded for small horizons.
  const policies: number[][] = [];
  const stack: number[] = [];
  const gen = (depth: number) => {
    if (depth === horizon) {
      policies.push(stack.slice());
      return;
    }
    for (let a = 0; a < actions; a++) {
      stack.push(a);
      gen(depth + 1);
      stack.pop();
    }
  };
  gen(0);
  const g = policies.map((p) => expectedFreeEnergy(model, p, precision));
  let best = 0;
  for (let i = 1; i < g.length; i++) if (g[i] > g[best]) best = i; // pymdp: maximize G
  return { policy: policies[best], efe: g };
}
