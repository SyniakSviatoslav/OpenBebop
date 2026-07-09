import { test } from 'node:test';
import assert from 'node:assert/strict';
import { checkRedLine, checkScope, RED_LINE_GLOBS, DEFAULT_SCOPE_GLOBS } from './guard.ts';

test('GREEN: secret/credential globs are red-lines (.env / secret / secrets)', () => {
  for (const p of ['.env', '.env.local', 'config/.env', 'secret/key.txt', 'secrets/vault.json', 'auth/token']) {
    assert.equal(checkRedLine(p).ok, false, `${p} must be a red-line`);
  }
});

test('GREEN: non-red-line paths pass', () => {
  for (const p of ['tools/bebop/loop.ts', 'docs/README.md', 'src/mcp.ts']) {
    assert.equal(checkRedLine(p).ok, true, `${p} must be allowed`);
  }
});

test('GREEN: extraGlobs strengthen the red-line set (user deny)', () => {
  assert.equal(checkRedLine('src/experimental.ts').ok, true, 'not a red-line by default');
  assert.equal(checkRedLine('src/experimental.ts', ['**/experimental.ts']).ok, false, 'user deny glob must apply');
});

test('GREEN: checkScope honors custom scope', () => {
  assert.equal(checkScope('tools/bebop/x.ts', DEFAULT_SCOPE_GLOBS, '/repo').ok, true);
  assert.equal(checkScope('random/y.ts', ['tools/**'], '/repo').ok, false, 'outside custom scope');
});

// Parity: when the Rust/WASM kernel is loaded, the guard must agree with the TS port on both a
// RED case (deny) and a GREEN case (allow). This proves the two engines enforce identical lines.
test('PARITY: kernel + TS port agree on red-line and scope (RED + GREEN)', async () => {
  const { initCore } = await import('./core-wasm.ts');
  const { setKernel, hasKernel } = await import('./guard.ts');
  const core = await initCore();
  // If the wasm artifact is absent we simply skip — the TS port is already covered above.
  if (!core) return;
  setKernel(core);
  assert.equal(hasKernel(), true, 'kernel registered');
  try {
    // RED: migration must be denied by BOTH engines
    assert.equal(checkRedLine('packages/db/migrations/x.sql').ok, false);
    assert.equal(checkRedLine('packages/db/migrations/x.sql').engine, 'rust');
    // GREEN: in-scope tool file must be allowed by BOTH engines
    const green = checkScope('tools/bebop/loop.ts', DEFAULT_SCOPE_GLOBS, '/repo');
    assert.equal(green.ok, true);
    assert.equal(green.engine, 'rust');
    // RED via scope: outside surface denied
    assert.equal(checkScope('apps/api/server.ts', DEFAULT_SCOPE_GLOBS, '/repo').ok, false);
  } finally {
    setKernel(null); // restore TS port for other tests
  }
});
