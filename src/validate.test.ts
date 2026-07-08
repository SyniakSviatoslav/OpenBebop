// Bebop validate tests — falsifiable RED+GREEN proofs of the boundary-layer contract.
//
// The pydantic principle under test: malformed external input is REJECTED at the wall, never
// silently patched. A GREEN case proves valid input passes; a RED case proves each required field
// is actually enforced (a missing/invalid field flips the verdict to failure, proving the contract
// is not a no-op).

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { validateToolArgs, type ValidationResult } from './validate.ts';

test('GREEN: a fully-specified valid tool call passes the boundary', () => {
  const r = validateToolArgs('edit', { path: 'a.ts', content: 'x=1' }) as Extract<ValidationResult, { ok: true }>;
  assert.equal(r.ok, true);
  assert.equal(r.name, 'edit');
  assert.equal(r.path, 'a.ts');
  assert.equal(r.content, 'x=1');
});

test('GREEN: "done" has no required fields and passes with empty args', () => {
  const r = validateToolArgs('done', {});
  assert.equal(r.ok, true);
});

test('RED: a missing required field is rejected (edit without content)', () => {
  const r = validateToolArgs('edit', { path: 'a.ts' }) as Extract<ValidationResult, { ok: false }>;
  assert.equal(r.ok, false);
  assert.match(r.reason, /content/);
});

test('RED: an empty-string required field is rejected (run with blank cmd)', () => {
  const r = validateToolArgs('run', { cmd: '' }) as Extract<ValidationResult, { ok: false }>;
  assert.equal(r.ok, false);
  assert.match(r.reason, /cmd/);
});

test('RED: an unknown tool name is rejected before any field check', () => {
  const r = validateToolArgs('rm -rf', { path: '/' }) as Extract<ValidationResult, { ok: false }>;
  assert.equal(r.ok, false);
  assert.equal(r.name, null); // never reached the contract
  assert.match(r.reason, /unknown tool/);
});

test('RED: unknown/extra fields are DROPPED, not forwarded (boundary is strict)', () => {
  const r = validateToolArgs('read', { path: 'a.ts', __proto__: { evil: 1 }, surprise: 'x' }) as Extract<ValidationResult, { ok: true }>;
  assert.equal(r.ok, true);
  assert.equal('surprise' in r, false); // extra field not in the typed payload
});

test('GREEN: every known tool name is accepted with its contract satisfied', () => {
  const cases: [string, Record<string, string>][] = [
    ['read', { path: 'a' }],
    ['grep', { pattern: 'x' }],
    ['edit', { path: 'a', content: 'b' }],
    ['run', { cmd: 'ls' }],
    ['dispatch', { task: 'build' }],
    ['done', {}],
  ];
  for (const [name, args] of cases) {
    const r = validateToolArgs(name, args);
    assert.equal(r.ok, true, `${name} should validate`);
  }
});
