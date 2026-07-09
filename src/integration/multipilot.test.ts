// multipilot.test.ts — brain-inside-brain tensor overlay, deterministic (RED+GREEN).
// Standing directive 2026-07-09: >=3 independent verifier loops, never silently averaged.

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { multipilot, defaultMultipilot, type AxisVerdict } from './multipilot.ts';

const A = (v: AxisVerdict) => async () => v;

test('GREEN: 3 independent axes all-approve → converged, promote', async () => {
  const r = await multipilot('artifact-x', [
    { axis: 'logical', verify: A('approve') },
    { axis: 'adversarial', verify: A('approve') },
    { axis: 'truth', verify: A('approve') },
  ]);
  assert.equal(r.overlay, 'converged');
  assert.equal(r.promote, true);
  assert.equal(r.action, 'promote');
  assert.deepEqual(r.vector, [1, 1, 1]);
});

test('RED: one axis rejects → divergent, NOT promoted (never averaged away)', async () => {
  const r = await multipilot('flawed-plan', [
    { axis: 'logical', verify: A('approve') },
    { axis: 'adversarial', verify: A('reject') }, // red-team catches it
    { axis: 'truth', verify: A('approve') },
  ]);
  assert.equal(r.overlay, 'divergent');
  assert.equal(r.promote, false);
  assert.equal(r.action, 'triage');
  assert.equal(r.dissent.length, 1);
  assert.equal(r.dissent[0].axis, 'adversarial');
});

test('RED: <3 loops is refused (independence is the point)', async () => {
  await assert.rejects(
    multipilot('x', [
      { axis: 'a', verify: A('approve') },
      { axis: 'b', verify: A('approve') },
    ]),
    /need >=3/,
  );
});

test('RED: duplicate axis names refused (loops must be independent)', async () => {
  await assert.rejects(
    multipilot('x', [
      { axis: 'a', verify: A('approve') },
      { axis: 'a', verify: A('approve') },
      { axis: 'b', verify: A('approve') },
    ]),
    /duplicate axis/,
  );
});

test('GREEN: defaultMultipilot wires logical+adversarial+truth as 3 distinct axes', async () => {
  const r = await defaultMultipilot('plan', {
    logical: A('approve'),
    adversarial: A('approve'),
    truth: A('revise'),
  });
  assert.equal(r.overlay, 'divergent');
  assert.equal(r.axes.length, 3);
  assert.ok(r.axes.every((a) => ['logical', 'adversarial', 'truth'].includes(a.axis)));
});
