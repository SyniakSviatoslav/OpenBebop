// logicalCot.test.ts — PDDL-INSTRUCT Logical CoT verifier, deterministic (RED+GREEN).
// arXiv:2509.13351 applied: structural (pre-computation) verification of each plan step.

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { verifyLogicalPlan, logicalChecker, type LogicalStep } from './logicalCot.ts';

type W = { branchClean: boolean; built: boolean; deployed: boolean; battery: number };
const start: W = { branchClean: true, built: false, deployed: false, battery: 100 };

const buildStep: LogicalStep<W> = {
  action: 'build',
  preconditions: { branchClean: true, built: false },
  effects: { built: true },
};
const deployStep: LogicalStep<W> = {
  action: 'deploy',
  preconditions: { built: true, branchClean: true },
  effects: { deployed: true },
  invariants: [{ name: 'branch-clean-at-deploy', holds: (s) => s.branchClean === true }],
};

test('GREEN: a valid plan replays end-to-end; trace records each state transition', () => {
  const p = verifyLogicalPlan(start, [buildStep, deployStep]);
  assert.equal(p.ok, true, p.message);
  assert.equal(p.trace.length, 2, 'one world snapshot per admitted step');
  assert.equal(p.world.deployed, true);
  assert.equal(p.world.built, true);
});

test('RED: precondition failure is caught structurally with a precise re-plan message', () => {
  // deploy BEFORE build → precondition built:true unmet in the start state
  const p = verifyLogicalPlan(start, [deployStep, buildStep]);
  assert.equal(p.ok, false);
  assert.equal(p.violation!.kind, 'precondition');
  assert.match(p.message, /cannot apply "deploy": precondition\(s\) unmet.*built/);
  assert.equal(p.trace.length, 0, 'no step admitted once the first fails (fail-closed)');
});

test('RED: invariant violation (dirty branch at deploy) is refused by the firewall', () => {
  const dirty: W = { ...start, built: true, branchClean: false };
  // preconditions of deploy require branchClean:true, so first the precondition fails —
  // use a step whose precondition passes but invariant breaks to isolate invariant logic.
  const forcePush: LogicalStep<W> = {
    action: 'force-deploy',
    preconditions: { built: true }, // satisfied
    effects: { deployed: true },
    invariants: [{ name: 'branch-clean-at-deploy', holds: (s) => s.branchClean === true }],
  };
  const p = verifyLogicalPlan(dirty, [forcePush]);
  assert.equal(p.ok, false);
  assert.equal(p.violation!.kind, 'invariant');
  assert.match(p.message, /breaks invariant\(s\): branch-clean-at-deploy/);
});

test('RED: an effect-noop (inert "action" claiming progress) is flagged', () => {
  const noop: LogicalStep<W> = { action: 'pretend-build', preconditions: { branchClean: true }, effects: { branchClean: true } };
  const p = verifyLogicalPlan(start, [noop]);
  assert.equal(p.ok, false);
  assert.equal(p.violation!.kind, 'effect-noop');
  assert.match(p.message, /effect-noop/);
});

test('GREEN: logicalChecker approves a valid plan; REVISES a broken one with the re-plan note', () => {
  const ok = logicalChecker(start, [buildStep, deployStep]);
  assert.equal(ok.verdict, 'approve');
  const bad = logicalChecker(start, [deployStep]);
  assert.equal(bad.verdict, 'revise');
  assert.match(bad.note, /precondition/);
});
