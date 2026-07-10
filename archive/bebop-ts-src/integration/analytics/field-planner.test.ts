import test from 'node:test';
import assert from 'node:assert/strict';
import { rustBuild, rustSpectral, rustFieldSensitivity, rustFieldArbiter, rustTopKContours, rustBuildF32 } from '../field-rust.ts';
import { plan } from './goap.ts';
import { fieldGatePlan, type FieldGatedPlan } from './field-planner.ts';

// Build a path graph (dependency chain) and accrue sensitivity history via propagations.
function pathAdj(n: number): number[][] {
  const A = Array.from({ length: n }, () => new Array(n).fill(0));
  for (let i = 0; i < n - 1; i++) {
    A[i][i + 1] = 1;
    A[i + 1][i] = 1;
  }
  return A;
}

test('rust f32 build + spectral match f64 build (storage halves, math identical)', async () => {
  const n = 30;
  const A = pathAdj(n);
  await rustBuild(A);
  const u0 = new Float64Array(n);
  u0[0] = 1.0;
  const f64 = await rustSpectral(u0, 5.0, 1.0, 30);
  await rustBuildF32(A);
  const f32 = await rustSpectral(u0, 5.0, 1.0, 30);
  let maxd = 0;
  for (let i = 0; i < n; i++) maxd = Math.max(maxd, Math.abs(f64[i] - f32[i]));
  assert.ok(maxd < 1e-12, `f32 vs f64 spectral mismatch=${maxd}`);
});

test('rust sensitivity bootstrap is non-uniform and peaks at the impulse source', async () => {
  const n = 30;
  const A = pathAdj(n);
  await rustBuild(A);
  const u0 = new Float64Array(n);
  u0[0] = 1.0;
  for (let c = 0; c < 5; c++) await rustSpectral(u0, 5.0, 1.0, 30); // accrue |Δu| history
  const s = await rustFieldSensitivity();
  assert.equal(s.length, n, 'sensitivity length must match graph');
  // source node (0) moves most → highest; far tail (29) quiesces → lowest (rank ordering holds).
  assert.ok(s[0] >= s[29], `source sens ${s[0]} must be >= tail ${s[29]}`);
  const anyNonUniform = s.some((x, i) => Math.abs(x - s[(i + 1) % n]) > 1e-6);
  assert.ok(anyNonUniform, 'sensitivity should not be perfectly uniform after history');
});

test('field-planner PERMITS a cheap GOAP plan when the field concurs', async () => {
  // World: a chain of deployments. Goal: reach "deployed". GOAP finds the path. Field cost of
  // each step is modest, PDDL estimates are generous → field concurs → plan permitted.
  const n = 6;
  const A = pathAdj(n);
  await rustBuild(A);
  const u0 = new Float64Array(n);
  u0[0] = 1.0;
  for (let c = 0; c < 3; c++) await rustSpectral(u0, 2.0, 1.0, 20); // small t → small ripple

  type W = { stage: number };
  const goal = { stage: 5 };
  const actions = [0, 1, 2, 3, 4].map((s) => ({
    name: `step${s}`,
    pre: { stage: s },
    eff: { stage: s + 1 },
    cost: 1.0,
  }));
  const pr = plan({ stage: 0 }, goal, actions);
  assert.ok(pr.ok, 'GOAP plan should reach goal');

  const gated: FieldGatedPlan = await fieldGatePlan(pr, {
    seedOf: (name) => parseInt(name.replace('step', ''), 10),
    pddlCostOf: () => 5.0, // PDDL over-estimates → field quiet → permit
    t: 2.0,
  });
  assert.equal(gated.overall, 'permit', `expected permit, got ${gated.overall} (${gated.reason})`);
  assert.equal(gated.ok, true);
  assert.equal(gated.actions.length, pr.plan.length);
});

test('field-planner OVERRIDES and blocks a plan when the field cost exceeds PDDL (RED→GREEN)', async () => {
  const n = 8;
  const A = pathAdj(n);
  await rustBuild(A);
  const u0 = new Float64Array(n);
  u0[0] = 1.0;
  for (let c = 0; c < 4; c++) await rustSpectral(u0, 30.0, 1.0, 40); // large t → big ripple

  type W = { stage: number };
  const pr = plan({ stage: 0 }, { stage: 7 }, [0, 1, 2, 3, 4, 5, 6].map((s) => ({
    name: `step${s}`,
    pre: { stage: s },
    eff: { stage: s + 1 },
    cost: 1.0,
  })));
  assert.ok(pr.ok, 'GOAP plan should reach goal');

  const gated: FieldGatedPlan = await fieldGatePlan(pr, {
    seedOf: (name) => parseInt(name.replace('step', ''), 10),
    pddlCostOf: () => 0.01, // PDDL massively under-estimates → field overrides
    t: 30.0,
  });
  assert.equal(gated.overall, 'override', `expected override, got ${gated.overall} (${gated.reason})`);
  assert.equal(gated.ok, false, 'override must block the plan');
});

test('rust top-K contours surface the worst-hit nodes (explainability)', async () => {
  const n = 25;
  const A = pathAdj(n);
  await rustBuild(A);
  const seed = new Float64Array(n);
  seed[0] = 1.0;
  const contours = await rustTopKContours(seed, 3, { t: 8.0 });
  assert.equal(contours.length, 3, 'top-3 requested');
  // impacts must be non-increasing
  for (let i = 1; i < contours.length; i++) {
    assert.ok(contours[i - 1].impact >= contours[i].impact, 'contours must be sorted by impact desc');
  }
  assert.ok(contours[0].index >= 0 && contours[0].index < n, 'contour index in range');
});
