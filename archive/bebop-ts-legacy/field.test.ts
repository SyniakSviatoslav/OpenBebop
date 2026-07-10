// Bebop field tests — falsifiable RED+GREEN proofs of the divergence/curl 3-state law.
//
// The load-bearing claim: ∇·F and (∇×F)_z are SEPARATE physical signals that trigger DIFFERENT
// directives. A purely radial (outward) traversal reads divergence-only → 'generate'; a purely
// tangential traversal reads curl-only → 'reconsider'; a diagonal (out + around) reads both.
// The RED cases prove each axis is independently necessary (removing the motion collapses it).

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { fieldState, directiveFor, searchFieldStateText, type FieldPoint } from './field.ts';

const P = (x: number, y: number, s = 1): FieldPoint => ({ x, y, s });

// ── PURE DIVERGENCE: a radial spoke — step is purely radial, no tangent ──
test('GREEN: a radial outward spoke has ∇·F>0 and ∇×F=0 → directive "generate"', () => {
  const a = fieldState([0, 0], [P(3, 0, 1), P(6, 0, 1)]);
  assert.ok(a.div > 0, `should diverge (∇·F=${a.div})`);
  assert.equal(a.curl, 0, `curl must be 0 (∇×F=${a.curl})`);
  assert.equal(a.state, 'diverge');
  assert.equal(a.directive, 'generate');
});

// ── PURE CURL: a tangential step at constant radius — step is perpendicular to radius ──
test('GREEN: a tangential step has (∇×F)_z≠0 and ∇·F=0 → directive "reconsider"', () => {
  const a = fieldState([0, 0], [P(6, 0, 1), P(6, 3, 1)]);
  assert.ok(a.curl !== 0, `should rotate (∇×F=${a.curl})`);
  assert.equal(a.div, 0, `divergence must be 0 (∇·F=${a.div})`);
  assert.equal(a.state, 'rotate');
  assert.equal(a.directive, 'reconsider');
});

// ── DIVERGENCE + CURL: a diagonal (outward + around) step hits BOTH axes ──
test('GREEN: a diagonal step (out+around) reads both axes → "generate+reconsider"', () => {
  const a = fieldState([0, 0], [P(6, 0, 1), P(8, 3, 1)]);
  assert.ok(a.div > 0, `diverges (∇·F=${a.div})`);
  assert.ok(a.curl !== 0, `rotates (∇×F=${a.curl})`);
  assert.equal(a.state, 'both');
  assert.equal(a.directive, 'generate+reconsider');
});

// ── RED: a pure-outward spoke has ZERO curl (removing the tangential motion kills rotation) ──
test('RED: the radial spoke has no curl — proves (∇×F)_z is a real, independent axis', () => {
  const a = fieldState([0, 0], [P(3, 0, 1), P(6, 0, 1)]);
  assert.equal(a.curl, 0, `radial motion yields zero curl (got ${a.curl})`);
  assert.notEqual(a.state, 'rotate');
});

// ── RED: a purely tangential step has ZERO divergence (removing radial motion kills divergence) ──
test('RED: the tangential step has no divergence — proves ∇·F is a real, independent axis', () => {
  const a = fieldState([0, 0], [P(6, 0, 1), P(6, 3, 1)]);
  assert.equal(a.div, 0, `tangential motion yields zero divergence (got ${a.div})`);
  assert.notEqual(a.state, 'diverge');
});

// ── RED: an inward spoke is a SINK (∇·F<0) → "focus", the opposite of divergence ──
test('RED: an inward spoke inverts the field to a sink → "focus" (divergence is signed)', () => {
  const a = fieldState([0, 0], [P(6, 0, 1), P(3, 0, 1)]); // flow toward query
  assert.ok(a.div < 0, `should be a sink (∇·F=${a.div})`);
  assert.equal(a.state, 'sink');
  assert.equal(a.directive, 'focus');
});

// ── VSA adapter: the law must hold in the embedding plane (searchFieldStateText) ──
test('GREEN: an exploratory VSA query produces a non-stable, action-guiding field state', () => {
  const a = searchFieldStateText('refactor the guard os', [
    'auth middleware', 'vector similarity', 'post quantum key', 'mesh gossip', 'landauer thermo',
  ]);
  assert.notEqual(a.state, 'stable', 'diverse candidates must produce a directional field');
  assert.ok(a.div < 0 || a.div > 0, `field must have a divergence sign (∇·F=${a.div})`);
});

test('RED: a fully-duplicate candidate field is STABLE → "focus" (no spurious generate/reconsider)', () => {
  const a = searchFieldStateText('guard', ['guard', 'guard', 'guard']);
  assert.equal(a.state, 'stable');
  assert.equal(a.directive, 'focus');
});

// ── directiveFor is a total, deterministic map of the 3-state law (+sink/stable degenerate) ──
test('GREEN: directiveFor is a total, deterministic map', () => {
  assert.equal(directiveFor('diverge'), 'generate');
  assert.equal(directiveFor('rotate'), 'reconsider');
  assert.equal(directiveFor('both'), 'generate+reconsider');
  assert.equal(directiveFor('sink'), 'focus');
  assert.equal(directiveFor('stable'), 'focus');
});
