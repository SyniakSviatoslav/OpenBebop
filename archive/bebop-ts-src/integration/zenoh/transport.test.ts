// src/integration/zenoh/transport.test.ts
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createLocalMesh, LocalMesh, type Envelope } from './transport.ts';

const env = (from: string, key: string, seq: number, priority = 128): Envelope => ({
  from, key, seq, priority,
  payload: new TextEncoder().encode(`${from}:${seq}`),
});

test('GREEN: pub/sub delivers across decentralized mesh (no central router)', () => {
  const [a, b, c] = createLocalMesh(['n1', 'n2', 'n3']);
  const received: string[] = [];
  const unsub = c.subscribe('l5/telemetry/**', (e) => received.push(e.from));
  const n = a.put(env('n1', 'l5/telemetry/n1', 1));
  // delivered to n1's local sub? n1 has none; to n3 via mesh gossip = 1; n2 no sub
  assert.equal(n, 0); // a has no subscriber locally
  assert.deepEqual(received, ['n1']); // c got it via mesh
  unsub();
});

test('GREEN: store/query returns last value by seq (Zenoh store union)', () => {
  const [a] = createLocalMesh(['n1']);
  a.put(env('n1', 'l5/state', 1));
  a.put(env('n1', 'l5/state', 2));
  const got = a.get('l5/state');
  assert.ok(got);
  assert.equal(got!.seq, 2); // highest seq wins deterministically
});

test('GREEN: priority arbitration — lower priority id wins (CAN-bus style)', () => {
  const [a, b] = createLocalMesh(['n1', 'n2']);
  // both write same key; lower priority should win the store
  a.put(env('n1', 'l5/lock', 1, 100));
  b.put(env('n2', 'l5/lock', 1, 50)); // higher priority (lower number)
  const stored = a.get('l5/lock');
  assert.ok(stored);
  assert.equal(stored!.from, 'n2'); // n2 won arbitration
});

test('RED: empty key is rejected (bad physics)', () => {
  const [a] = createLocalMesh(['n1']);
  assert.throws(() => a.put(env('n1', '', 1)), /key required/);
});

test('RED: priority out of range is rejected', () => {
  const [a] = createLocalMesh(['n1']);
  assert.throws(() => a.put(env('n1', 'x', 1, 256)), /priority out of range/);
  assert.throws(() => a.put(env('n1', 'x', 1, -1)), /priority out of range/);
});

test('RED: negative seq rejected (monotonicity invariant)', () => {
  const [a] = createLocalMesh(['n1']);
  assert.throws(() => a.put(env('n1', 'x', -1)), /seq must be >= 0/);
});
