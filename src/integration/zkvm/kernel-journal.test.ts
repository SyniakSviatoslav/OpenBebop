// src/integration/zkvm/kernel-journal.test.ts
//
// RED+GREEN: zkVM decide() as a tamper-evident kernel journal. GREEN = deterministic digest that
// verifies; RED = a tampered stored digest or flipped input fails verification.

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { journalize, verifyJournal, serializeState } from './kernel-journal.ts';

function bytesEqual(a: Uint8Array, b: Uint8Array): boolean {
  if (a.length !== b.length) return false;
  for (let i = 0; i < a.length; i++) if (a[i] !== b[i]) return false;
  return true;
}

function st() {
  return { ingested: new Set(['a']), published: new Set(['b']), revoked: new Set([]), seen: new Set(['c']), lastBackend: 'native' as const };
}

test('GREEN: journalize is deterministic and verifies against itself', () => {
  const d1 = journalize(st(), 'cmdhash1', 1);
  const d2 = journalize(st(), 'cmdhash1', 1);
  assert.ok(bytesEqual(d1, d2), 'same input → same digest');
  assert.ok(verifyJournal(st(), 'cmdhash1', 1, d1), 'self-verify must pass');
});

test('GREEN: different counter yields a different digest (sequence binding)', () => {
  const d1 = journalize(st(), 'cmdhash1', 1);
  const d2 = journalize(st(), 'cmdhash1', 2);
  assert.ok(!bytesEqual(d1, d2), 'counter must change digest');
});

test('RED: a tampered stored digest fails verification', () => {
  const d = journalize(st(), 'cmdhash1', 1);
  const tampered = d.slice();
  tampered[0] ^= 0xff; // flip one byte
  assert.ok(!verifyJournal(st(), 'cmdhash1', 1, tampered), 'tampered digest must fail');
});

test('RED: a different command hash fails verification', () => {
  const d = journalize(st(), 'cmdhash1', 1);
  assert.ok(!verifyJournal(st(), 'cmdhash1-other', 1, d), 'mismatched command hash must fail');
});

test('GREEN: serializeState is order-independent (set iteration order does not matter)', () => {
  const a = serializeState({ ingested: new Set(['x', 'y']), published: new Set(), revoked: new Set(), seen: new Set(), lastBackend: null });
  const b = serializeState({ ingested: new Set(['y', 'x']), published: new Set(), revoked: new Set(), seen: new Set(), lastBackend: null });
  assert.ok(bytesEqual(a, b), 'set order must not affect serialization');
});
