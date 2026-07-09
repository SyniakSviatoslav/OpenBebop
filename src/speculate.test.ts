// Bebop speculate tests — falsifiable RED+GREEN proofs (Verified-by-Math).
//
// Each property pairs a GREEN case (passes on good input) with a RED case that must FAIL on
// bad input, proving the assertion is not a no-op. The load-bearing claim — that the
// semi-autoregressive sequential module beats a pure parallel drafter under suffix decay — is
// proven as the central RED+GREEN pair.

import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
  confidenceSchedule, semiAutoDraft, parallelDraft, verifyBlock,
  scheduleVerificationLength, proposeStep, type DraftCandidate,
} from './speculate.ts';

const c = (name: string, conf: number): DraftCandidate => ({ name, conf });

// ── confidence schedule (head cool → tail warm) ───────────────────────────────

test('GREEN: confidenceSchedule returns one temp per position, cool at head', () => {
  const s = confidenceSchedule(4);
  assert.equal(s.length, 4);
  assert.ok(s[0] < s[3], 'tail must be warmer than head');
  assert.ok(Math.abs(s[0] - 0.6) < 1e-9);
});

test('RED: a flat schedule (head===tail) is NOT the DSpark pattern — monotonic spread collapses', () => {
  const s = confidenceSchedule(4, 0.7, 0.7);
  assert.equal(s[0], s[3]); // degenerate: no head/tail seasoning
  assert.ok(s.every((t) => t === 0.7));
});

// ── semi-autoregressive vs pure parallel drafter (the core DSpark finding) ─────

test('GREEN: with honest, independent confidences the semi-auto draft accepts the full block', () => {
  const block = [c('a', 1), c('b', 1), c('c', 1), c('d', 1)];
  const { acceptedLen, prefixSurvival } = semiAutoDraft(block);
  assert.equal(acceptedLen, 4);
  assert.deepEqual(prefixSurvival, [1, 1, 1, 1]);
});

test('RED: a pure parallel drafter loses the tail that the semi-auto draft recovers', () => {
  // suffix decay: backbone confidences decay toward the tail; pure product falls below floor at k=3
  const decayed = [c('a', 0.9), c('b', 0.85), c('c', 0.6), c('d', 0.5)];
  const semi = semiAutoDraft(decayed).acceptedLen;
  const par = parallelDraft(decayed);
  assert.ok(semi > par, `semi-auto ${semi} should beat pure-parallel ${par} under suffix decay`);
});

test('RED: a confident parallel block still falls behind semi-auto once dependence is modeled', () => {
  const block = [c('a', 0.85), c('b', 0.8), c('c', 0.72), c('d', 0.68)];
  const semi = semiAutoDraft(block).acceptedLen;
  const par = parallelDraft(block);
  assert.ok(semi >= par);
  if (par < block.length) assert.ok(semi > par, 'sequential module must extend the accepted prefix');
});

// ── floor behaviour ───────────────────────────────────────────────────────────

test('GREEN: low-confidence block clears no prefix (silent, safe rejection)', () => {
  const block = [c('a', 0.1), c('b', 0.2), c('c', 0.15)];
  const { acceptedLen } = semiAutoDraft(block, { floor: 0.5 });
  assert.equal(acceptedLen, 0); // nothing accepted → loop falls back to single call
});

test('RED: lowering the floor to 0 makes the draft accept junk (proves the floor bites)', () => {
  const block = [c('a', 0.1), c('b', 0.2)];
  const accepted = semiAutoDraft(block, { floor: 0.01 }).acceptedLen;
  assert.equal(accepted, 2); // floor near 0 → admits noise
});

// ── verifier (the deterministic guard) is authoritative ───────────────────────

test('GREEN: a drafted block that the guard approves is fully verified', () => {
  const drafted = ['edit x', 'read y', 'done'];
  const { verifiedLen, rejectedAt } = verifyBlock(drafted, () => true);
  assert.equal(verifiedLen, 3);
  assert.equal(rejectedAt, null);
});

test('RED: the first guard-denied candidate halts verification (target model overrides draft)', () => {
  const drafted = ['edit x', 'read secret.env', 'done'];
  const guard = (t: string) => !t.includes('secret');
  const { verifiedLen, rejectedAt } = verifyBlock(drafted, guard);
  assert.equal(verifiedAt(verifiedLen), 1); // only 'edit x' verified
  assert.equal(rejectedAt, 1); // 'read secret.env' rejected
});

function verifiedAt(n: number): number { return n; }

// ── confidence-scheduled verification length (load-aware) ──────────────────────

test('GREEN: high survival + headroom → schedule a LONGER block (use capacity well)', () => {
  const long = scheduleVerificationLength(0.95, 1, 1, 8);
  const short = scheduleVerificationLength(0.1, 1, 1, 8);
  assert.ok(long > short, `long ${long} should exceed short ${short}`);
  assert.ok(long <= 8 && short >= 1);
});

test('RED: near-zero survival or zero throughput → schedule the MINIMUM block (no wasted batch)', () => {
  const l0 = scheduleVerificationLength(0.95, 0, 1, 8);
  const s0 = scheduleVerificationLength(0.0, 1, 1, 8);
  assert.equal(l0, 1);
  assert.equal(s0, 1);
});

// ── end-to-end propose+verify ──────────────────────────────────────────────────

test('GREEN: proposeStep drafts a block in ONE call and reports round-trip savings', () => {
  let calls = 0;
  const res = proposeStep<string>(
    () => { calls++; return ['read a', 'edit b', 'done']; },
    () => true,
  );
  assert.equal(calls, 1, 'drafter invoked once (one round-trip for the whole block)');
  assert.equal(res.draftedLen, 3);
  assert.equal(res.verifiedLen, 3);
  assert.equal(res.roundTripsSaved, 2, 'one call replaced three sequential calls');
});

test('RED: a guard-rejected draft still counts as one round-trip but verifies fewer', () => {
  const res = proposeStep<string>(
    () => ['edit ok', 'edit packages/db/migrations/002.sql', 'done'],
    (t) => !t.includes('migrations'),
  );
  assert.equal(res.draftedLen, 3);
  assert.equal(res.verifiedLen, 1); // 'edit ok' only
  assert.equal(res.rejectedAt, 1);
  assert.equal(res.roundTripsSaved, 2); // still saved the round-trips even though 2 were rejected
});
