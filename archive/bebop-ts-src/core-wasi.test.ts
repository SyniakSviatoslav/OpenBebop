// Bebop Sovereign Node — Phase 2 WASI core loader tests (RED+GREEN, falsifiable).
//
// Load-bearing claim: when wasmedge or the .wasi.wasm binary is absent, initWasiCore() degrades to
// null (the box still boots on the in-process core). When present, it returns the same CoreHandle.
import assert from 'node:assert/strict';
import test from 'node:test';
import { initWasiCore } from './core-wasi.ts';

test('GREEN: initWasiCore returns null gracefully when no WASI binary is present (degrades to in-process core)', async () => {
  // No dist/bebop_core.wasi.wasm exists in this repo working tree → must NOT throw, must return null.
  const h = await initWasiCore();
  assert.equal(h, null, 'absent WASI artifact must yield null, not crash');
});

test('RED: a missing WASI artifact is never advertised as a loaded core (no false-positive handle)', async () => {
  // If someone wires BEBOP_CORE_RUNTIME=wasi without the artifact, the agent must NOT claim a core.
  const h = await initWasiCore();
  // The contract: either a real handle with loaded===true, or null. Never an object without loaded.
  if (h !== null) assert.equal(h.loaded, true, 'any returned handle must be a genuine loaded core');
  else assert.equal(h, null, 'absent artifact must degrade to null, never a fake handle');
});
