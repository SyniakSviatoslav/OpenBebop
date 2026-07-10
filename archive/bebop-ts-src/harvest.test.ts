// Bebop harvest tests — falsifiable RED+GREEN proofs the memory-mining tool finds skills + patterns.
//
// Load-bearing claim: the agent can RUN its own tools on its living-memory history to (a) surface
// skill candidates and (b) detect first-order + cross-patterns. A thin memory yields NOTHING (no
// fabrication); a structured memory yields classified patterns. We prove both.

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { livingMemory } from './memory.ts';
import { harvest, candidateSkills, patternMap, crossPatterns, recallField } from './harvest.ts';
import type { Skill } from './skills.ts';

// Seed a deterministic, structured memory: a spreading neighbourhood + a cyclic one.
function seed(): string[] {
  const m = livingMemory();
  m.clear();
  // "field" fans out into related concepts (divergence candidate) and links back (cycle)
  const f = m.remember('field', 'vector field', undefined, { layer: 'long' });
  const d = m.remember('divergence', 'net outward flow', [f], { layer: 'long' });
  const c = m.remember('curl', 'rotation', [f], { layer: 'long' });
  m.rememberLink(f, d); m.rememberLink(f, c); m.rememberLink(d, c); // triangle = both/cycle
  m.remember('gradient', 'slope', [f], { layer: 'long' });
  m.remember('laplacian', 'divergence of gradient', [d], { layer: 'long' });
  return ['field', 'divergence', 'curl', 'gradient', 'laplacian'];
}

test('GREEN: a structured concept fans into ≥3 concepts (spread detected, no thin-memory false-positive)', () => {
  const concepts = seed();
  const hits = recallField('field');
  assert.ok(hits.length >= 2, 'field should recall neighbours');
  const cand = candidateSkills(concepts);
  const fieldCand = cand.find((x) => x.concept === 'field');
  assert.ok(fieldCand, 'field should be a skill candidate (fans into ≥3)');
  assert.ok(fieldCand!.field.spread >= 3, `spread ${fieldCand!.field.spread} >= 3`);
});

test('RED: an ISOLATED concept (no recall) produces NO skill candidate (no fabrication on thin memory)', () => {
  const m = livingMemory(); m.clear();
  m.remember('orphan', 'x', undefined, { layer: 'long' });
  const cand = candidateSkills(['orphan']);
  assert.equal(cand.length, 0, 'orphan with no recall must NOT become a skill');
});

test('GREEN: recallField enriches with vector-similarity nearest() when content-address recall is thin', () => {
  const m = livingMemory(); m.clear();
  // "govern" and "governor" are NOT linked, so content-address recall yields little; nearest() should
  // still surface the associative neighbour.
  m.remember('governor', 'L5 telemetry', undefined, { layer: 'long' });
  const hits = recallField('govern');
  assert.ok(hits.some((h) => h.concept === 'governor'), 'associative nearest() should find governor');
});

test('GREEN: a query matching an existing concept returns that concept via associative recall', () => {
  const m = livingMemory(); m.clear();
  m.remember('governor', 'L5 telemetry', undefined, { layer: 'long' });
  const hits = recallField('governor'); // exact match → returns itself
  assert.ok(hits.some((h) => h.concept === 'governor'), 'exact concept should recall itself');
});

test('RED: an EMPTY memory yields no recall field (honest, no fabrication)', () => {
  const m = livingMemory(); m.clear();
  const hits = recallField('anything-at-all');
  assert.equal(hits.length, 0, 'empty store must surface nothing');
});

test('GREEN: patternMap classifies a structured corpus into non-isolated field classes', () => {
  const concepts = seed();
  const pats = patternMap(concepts);
  const fieldPat = pats.find((p) => p.concept === 'field')!;
  assert.notEqual(fieldPat.kind, 'isolated', 'field with neighbours must NOT read as isolated');
  // the triangle yields real structure: at least one concept is a classified (non-isolated) field
  assert.ok(pats.some((p) => p.kind !== 'isolated'), 'structured corpus yields classified patterns');
});

test('RED: an empty corpus yields only "isolated" patterns (no invented structure)', () => {
  const m = livingMemory(); m.clear();
  const pats = patternMap(['ghost', 'phantom']);
  assert.ok(pats.every((p) => p.kind === 'isolated'), 'no memory → all isolated');
});

test('GREEN: crossPatterns surface the explore→reflect coupling in a structured corpus', () => {
  const concepts = seed();
  const cross = crossPatterns(concepts);
  assert.ok(cross.length >= 1, 'should detect at least one coupling');
  // the dominant coupling should be a real recall-overlap, not noise
  assert.ok(cross[0].count >= 1);
});

test('GREEN: harvest() report bundles candidates + patterns + cross + existing skills', () => {
  const concepts = seed();
  // Inject a deterministic skill list so the assertion does not depend on which skills happen to
  // be installed on the local machine (~/.hermes/skills/ differs per environment → was flaky on CI,
  // where 'review' was absent and the test failed). harvest() accepts an optional skills param.
  const skills: Skill[] = [
    { name: 'review', description: 'code review', body: '', dir: '/tmp/skills/review' },
    { name: 'deploy', description: 'deploy', body: '', dir: '/tmp/skills/deploy' },
  ];
  const rep = harvest(concepts, skills);
  assert.ok(Array.isArray(rep.candidates));
  assert.ok(Array.isArray(rep.patterns));
  assert.ok(Array.isArray(rep.cross));
  assert.deepEqual(rep.existingSkills.sort(), ['deploy', 'review'], 'injected skills are listed as existing');
});
