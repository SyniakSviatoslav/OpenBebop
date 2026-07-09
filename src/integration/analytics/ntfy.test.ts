// N8b ntfy alert sink — deterministic delivery shape (RED+GREEN).

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { shouldAlert, notifyNtfy, governorAlertNtfy, type NtfyConfig } from './ntfy.ts';

const cfg: NtfyConfig = { baseUrl: 'https://ntfy.sh', topic: 'bebop-ops', title: 'bebop' };

test('GREEN: healthy governor state → no alert', () => {
  assert.equal(shouldAlert({ degradationSignal: false, safeState: false, hallucinationRate: 0, cycleBroken: false, pcaAnomaly: false }), null);
  assert.equal(governorAlertNtfy(cfg, { hallucinationRate: 0 }), null);
});

test('RED: safe-state trip → alert kind safe-state', () => {
  assert.equal(shouldAlert({ safeState: true }), 'safe-state');
  const req = governorAlertNtfy(cfg, { safeState: true })!;
  assert.equal(req.method, 'POST');
  assert.equal(req.url, 'https://ntfy.sh/bebop-ops');
  assert.ok(req.body.includes('safe-state'));
  assert.equal(req.headers.Title, 'bebop');
});

test('RED: degradation signal → alert kind degradation', () => {
  assert.equal(shouldAlert({ degradationSignal: true, hallucinationRate: 0.6 }), 'degradation');
});

test('RED: hallucination spike ≥0.5 → alert', () => {
  assert.equal(shouldAlert({ hallucinationRate: 0.7 }), 'hallucination-spike');
});

test('GREEN: notifyNtfy builds the exact POST shape (pure, caller fetches)', () => {
  const req = notifyNtfy(cfg, 'cycle-broken', 'module X dropped a field');
  assert.equal(req.url, 'https://ntfy.sh/bebop-ops');
  assert.equal(req.headers['Content-Type'], 'text/plain');
  assert.equal(req.headers.Tags, 'cycle-broken');
  assert.ok(req.body.includes('module X dropped a field'));
});
