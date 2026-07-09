// N8c GOAP — Goal-Oriented Action Planning, deterministic (RED+GREEN).

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { plan, actionAllowed, type GoapAction, type WorldState } from './goap.ts';

interface DevWorld extends WorldState {
  branch: string;
  tests: 'fail' | 'pass';
  deployed: boolean;
  charged: boolean;
}

const actions: GoapAction<DevWorld>[] = [
  { name: 'write-code', pre: { tests: 'fail' }, eff: { tests: 'pass' }, cost: 1 },
  { name: 'run-tests', pre: { tests: 'fail' }, eff: { tests: 'pass' }, cost: 1 },
  {
    name: 'deploy',
    pre: { tests: 'pass', branch: 'feat/x' },
    eff: { deployed: true },
    cost: 5,
    invariant: (s) => s.branch === 'feat/x', // symbolic firewall: never deploy a dirty main
  },
];

test('GREEN: reachable goal returns a plan (advisor names goal, kernel plans)', () => {
  const start: DevWorld = { branch: 'feat/x', tests: 'fail', deployed: false, charged: true };
  const r = plan(start, { tests: 'pass' }, actions);
  assert.equal(r.ok, true);
  assert.deepEqual(r.plan, ['write-code']);
});

test('GREEN: already-satisfied goal → empty plan, ok', () => {
  const start: DevWorld = { branch: 'feat/x', tests: 'pass', deployed: false, charged: true };
  const r = plan(start, { tests: 'pass' }, actions);
  assert.equal(r.ok, true);
  assert.equal(r.plan.length, 0);
});

test('RED: unreachable goal (unsatisfied precondition) → no path, kernel cannot hallucinate', () => {
  // to deploy we need tests:pass on branch feat/x; but the advisor set goal deployed:true from a
  // failing main branch — the invariant + preconditions make it UNREACHABLE.
  const start: DevWorld = { branch: 'main', tests: 'fail', deployed: false, charged: true };
  const r = plan(start, { deployed: true }, actions);
  assert.equal(r.ok, false);
  assert.equal(r.reason, 'unreachable');
  assert.equal(r.plan.length, 0);
});

test('RED: invariant firewall blocks an action even when preconditions pass', () => {
  // tests pass on main, but deploy invariant (branch===feat/x) must refuse — the kernel rejects it.
  const start: DevWorld = { branch: 'main', tests: 'pass', deployed: false, charged: true };
  const deploy = actions.find((a) => a.name === 'deploy')!;
  assert.equal(actionAllowed(start, deploy), false, 'kernel must refuse deploy on main (firewall)');
});

test('GREEN: actionAllowed true when pre + invariant both hold', () => {
  const start: DevWorld = { branch: 'feat/x', tests: 'pass', deployed: false, charged: true };
  const deploy = actions.find((a) => a.name === 'deploy')!;
  assert.equal(actionAllowed(start, deploy), true);
});
