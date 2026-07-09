// src/integration/wetware/finalspark.test.ts
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { LocalWetwareStub, FinalSparkAdapter, type Stimulus } from './finalspark.ts';

test('GREEN: deterministic stub emits a spike when threshold crossed', async () => {
  const w = new LocalWetwareStub();
  const s: Stimulus = { channel: 3, amplitude: 30, duration: 10 }; // >20 threshold -> spike
  const spikes = await w.apply(s);
  assert.equal(spikes.length, 1);
  assert.equal(spikes[0].channel, 3);
});

test('GREEN: deterministic — same stimulus yields identical spikes', async () => {
  const w1 = new LocalWetwareStub();
  const w2 = new LocalWetwareStub();
  const s: Stimulus = { channel: 1, amplitude: 30, duration: 10 };
  assert.deepEqual(await w1.apply(s), await w2.apply(s));
});

test('RED: amplitude beyond bio-safe limit rejected', async () => {
  const w = new LocalWetwareStub();
  await assert.rejects(() => w.apply({ channel: 0, amplitude: 999, duration: 5 }), /bio-safe limit/);
});

test('RED: negative channel rejected', async () => {
  const w = new LocalWetwareStub();
  await assert.rejects(() => w.apply({ channel: -1, amplitude: 10, duration: 5 }), /channel < 0/);
});

test('RED: remote adapter refuses without key (honest integration point)', () => {
  assert.throws(() => new FinalSparkAdapter(''), /API key required/);
});
