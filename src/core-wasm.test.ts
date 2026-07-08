import { test } from 'node:test';
import assert from 'node:assert/strict';
import { initCore, getCore } from './core-wasm.ts';

test('RED: wasm kernel denies a migration (red-line)', async () => {
  const core = await initCore();
  assert.ok(core, 'bebop_core.wasm should load in tests');
  if (!core) return;
  const d = core.decide('packages/db/migrations/002_users.sql', 'edit');
  assert.equal(d.ok, false, 'migrations are red-line → denied');
  assert.equal(d.kind, 'redline');
});

test('GREEN: wasm kernel allows in-scope tool files', async () => {
  const core = await initCore();
  if (!core) return;
  const d = core.decide(
    'tools/bebop/src/loop.ts',
    'edit',
    [],
    ['tools/bebop/**', 'docs/design/dowiz-agent-cli/**'],
    '/repo',
  );
  assert.equal(d.ok, true, 'in-scope tool file → pass');
  assert.equal(d.kind, 'ok');
});

test('wasm kernel embeds deterministically and ranks similar > dissimilar', async () => {
  const core = await initCore();
  if (!core) return;
  const a = core.embed('the red ship lifts off', 256);
  const b = core.embed('the red ship lifts off', 256);
  const c = core.embed('unrelated coffee morning', 256);
  assert.deepEqual(a, b);
  assert.ok(core.similarity(a, b) > core.similarity(a, c));
});

test('estimateTokens counts up with length', async () => {
  const core = await initCore();
  if (!core) return;
  assert.equal(core.estimateTokens(''), 0);
  assert.ok(core.estimateTokens('a'.repeat(400)) > 0);
});

test('getCore returns the cached handle', async () => {
  const core = await initCore();
  assert.equal(getCore(), core);
});
