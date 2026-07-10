// modelGateway.test.ts — Portkey-method normalized gateway seam, deterministic (RED+GREEN).

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { gatewayRoute, type GatewayConfig } from './modelGateway.ts';

const cfg: GatewayConfig = {
  keys: {
    haiku: { id: 'vk-haiku', provider: 'openrouter', model: 'haiku' },
    sonnet: { id: 'vk-sonnet', provider: 'anthropic', model: 'sonnet' },
    opus: { id: 'vk-opus', provider: 'anthropic', model: 'opus' },
  },
  fallback: ['sonnet', 'haiku'],
  guardrails: true,
};

test('GREEN: a doer task routes to haiku via its virtual key, with fallback chain', () => {
  const p = gatewayRoute({ taskClass: 'doer' }, cfg);
  assert.equal(p.ok, true);
  assert.equal(p.primary!.model, 'haiku');
  assert.equal(p.primary!.keyId, 'vk-haiku');
  assert.deepEqual(p.fallback.map((f) => f.model), ['sonnet']);
});

test('GREEN: a red-line task routes to opus (guardrail passes)', () => {
  const p = gatewayRoute({ taskClass: 'redline' }, cfg);
  assert.equal(p.ok, true);
  assert.equal(p.primary!.model, 'opus');
  assert.equal(p.primary!.keyId, 'vk-opus');
});

test('RED: guardrails REFUSE to forward a red-line task to a non-opus lane (fail-closed)', () => {
  const bad = gatewayRoute({ taskClass: 'redline', forceModel: 'haiku' }, cfg);
  assert.equal(bad.ok, false, 'kernel must not forward money/auth to haiku');
  assert.match(bad.note, /routing violation|opus/i);
  assert.equal(bad.primary, undefined, 'no plan emitted for a forbidden route');
});

test('RED: missing virtual key → no fabricate, plan refused', () => {
  const noKey: GatewayConfig = { keys: { haiku: cfg.keys.haiku! }, guardrails: true };
  const p = gatewayRoute({ taskClass: 'reason' }, noKey); // reason→sonnet, but no sonnet key
  assert.equal(p.ok, false, 'a lane with no configured key must be refused, not faked');
  assert.match(p.note, /no virtual key/i);
});
