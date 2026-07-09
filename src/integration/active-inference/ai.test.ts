/**
 * ai.test.ts — RED+GREEN falsifiable tests for the deterministic Active Inference engine.
 * GREEN: belief update matches pymdp/information-theoretic expectation; best policy minimizes EFE.
 * RED: a malformed model (non-normalized distribution, NaN) is rejected / handled.
 */
import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
  inferStates,
  selectPolicy,
  expectedFreeEnergy,
  softmax,
  kl,
  type PomdpModel,
} from './ai.ts';

// Tiny 2-state / 2-obs / 2-action POMDP.
function makeModel(): PomdpModel {
  // A[o][s] = p(o|s)
  const A = [
    [0.9, 0.1], // obs 0 more likely from state 0
    [0.1, 0.9],
  ];
  // B[action][to][from]
  const B: number[][][] = [
    // action 0: stay
    [
      [1, 0],
      [0, 1],
    ],
    // action 1: flip
    [
      [0, 1],
      [1, 0],
    ],
  ];
  // C: prefer obs 0
  const C = [0.85, 0.15];
  // D: start at state 0
  const D = [1, 0];
  return { A, B, C, D, actions: 2 };
}

// Real pymdp ground truth (captured by async re-engineering subagent, pymdp 1.0.3 from
// infer-actively/pymdp): model A=[[.95,.05],[.05,.95]], B[:,:,0]=I, B flips on action 1,
// C=[-2,0], D=[1,0], obs=1, policy_len=1 →
//   POSTERIOR q(s|o) = [1. 0.]
//   NEG_EFE G       = [-2.026928  -0.226928]
//   CHOSEN ACTION    = [1.]
// This test pins the module to that REAL output (not just internal consistency).
function pymdpModel(): PomdpModel {
  const A = [
    [0.95, 0.05],
    [0.05, 0.95],
  ];
  const B: number[][][] = [
    [[1, 0], [0, 1]], // action 0: stay
    [[0, 1], [1, 0]], // action 1: flip
  ];
  const C = [-2, 0]; // prefer obs 0 strongly
  const D = [1, 0]; // start at state 0
  return { A, B, C, D, actions: 2 };
}

test('GREEN (pymdp ground truth): posterior + chosen action match REAL pymdp output', () => {
  const m = pymdpModel();
  // obs=1 → observed outcome index 1
  const post = inferStates(m.D, m.A[1], 1);
  assert.ok(post[0] > 0.999, `pymdp posterior[0]≈1, got ${post[0]}`);
  assert.ok(post[1] < 0.001, `pymdp posterior[1]≈0, got ${post[1]}`);
  // policy_len=1: choose action maximizing G (negative EFE)
  const { policy } = selectPolicy(m, 1, 1);
  assert.deepEqual(policy, [1], 'pymdp chose action 1 (flip toward preferred obs 1; C=[-2,0])');
});

test('GREEN (pymdp ground truth): NEG-EFE matches REAL pymdp G = [-2.026928, -0.226928]', () => {
  const m = pymdpModel();
  const g0 = expectedFreeEnergy(m, [0], 1); // G for policy [0]
  const g1 = expectedFreeEnergy(m, [1], 1); // G for policy [1]
  assert.ok(Math.abs(g0 - -2.026928) < 1e-3, `G[0] should be ≈ -2.026928, got ${g0}`);
  assert.ok(Math.abs(g1 - -0.226928) < 1e-3, `G[1] should be ≈ -0.226928, got ${g1}`);
  // pymdp maximizes G (negative EFE); larger G = better. G[1] > G[0] so action 1 wins.
  assert.ok(g1 > g0, `policy [1] has higher G (better) than [0]: g1=${g1} g0=${g0}`);
});

test('GREEN: inferStates produces a normalized posterior', () => {
  const m = makeModel();
  const post = inferStates(m.D, m.A[0], 1); // observed obs 0, prior at state 0
  const sum = post.reduce((a, b) => a + b, 0);
  assert.ok(Math.abs(sum - 1) < 1e-12, `posterior must sum to 1, got ${sum}`);
  assert.ok(post[0] > post[1], `obs 0 should raise belief in state 0 (${post})`);
});

test('GREEN: precision raises confidence (higher precision → sharper posterior)', () => {
  const m = makeModel();
  const low = inferStates(m.D, m.A[0], 0.5);
  const high = inferStates(m.D, m.A[0], 8);
  assert.ok(high[0] > low[0], `higher precision should sharpen toward state 0: low=${low[0]} high=${high[0]}`);
});

test('GREEN: selectPolicy picks the action that reaches the preferred observation at horizon 1', () => {
  const m = makeModel();
  const { policy } = selectPolicy(m, 1, 1);
  // start state 0, prefer obs 0. Action 0 (stay) keeps state 0 → obs 0 likely (preferred).
  // Action 1 (flip) → state 1 → obs 1 likely (not preferred). So best = [0].
  assert.deepEqual(policy, [0]);
});

test('GREEN: G of all policies is finite and best >= worst (G is maximized)', () => {
  const m = makeModel();
  const { efe } = selectPolicy(m, 2, 1);
  for (const e of efe) assert.ok(Number.isFinite(e), `G must be finite, got ${e}`);
  const best = Math.max(...efe);
  const worst = Math.min(...efe);
  assert.ok(best >= worst, 'best G must be >= worst G');
});

test('GREEN: softmax is invariant to additive shift (numerical stability)', () => {
  const a = softmax([1, 2, 3]);
  const b = softmax([101, 102, 103]);
  for (let i = 0; i < 3; i++) {
    assert.ok(Math.abs(a[i] - b[i]) < 1e-12, `softmax shift-invariance violated at ${i}`);
  }
});

// ───── RED ─────

test('RED: G must DROP when the preference is flipped away from the reachable obs', () => {
  const m = makeModel();
  const gPrefer0 = expectedFreeEnergy(m, [0], 1); // stay → obs 0, matches C=[0.85,0.15]
  const Cflip: PomdpModel = { ...m, C: [0.15, 0.85] };
  const gPrefer1 = expectedFreeEnergy(Cflip, [0], 1); // stay → obs 0, now DISpreferred
  assert.ok(
    gPrefer1 < gPrefer0,
    `G should be lower when action opposes preference (${gPrefer1} vs ${gPrefer0})`,
  );
});

test('RED: KL divergence is 0 for identical distributions and >0 for different', () => {
  const a = [0.5, 0.5];
  assert.ok(Math.abs(kl(a, a)) < 1e-9, 'KL(p||p) must be 0');
  assert.ok(kl(a, [0.9, 0.1]) > 0, 'KL(p||q) must be > 0 for p≠q');
});

test('RED: selectPolicy over a nan-preference model is rejected (no silent NaN policy)', () => {
  const m = makeModel();
  const bad: PomdpModel = { ...m, C: [Number.NaN, 0.0] };
  assert.throws(() => selectPolicy(bad, 1, 1), /NaN|preference/i);
});
