import { test } from 'node:test';
import assert from 'node:assert/strict';
import { recall, estimateTokens } from './knowledge.ts';

// The living-knowledge retriever + VSA cli are NOT bundled in this standalone repo.
// recall() must (a) use the BUNDLED in-process living memory (VSA + graph), returning REAL payload
// text + a similarity score, and (b) degrade honestly (no spawn, no fabricated hits) when the
// richer §0·GP retriever is absent.

test('GREEN: recall returns REAL payload text + VSA score from bundled memory (not truncated ids)', () => {
  const r = recall('kernel law');
  assert.ok(r.hits.length > 0, 'should find seeded corpus nodes');
  for (const h of r.hits) {
    assert.ok(h.text.length > 12, `hit text should be the real payload, got: ${JSON.stringify(h.text)}`);
    assert.ok(typeof h.score === 'number' && h.score > 0, `hit should carry a score, got: ${h.score}`);
  }
});

test('GREEN: exact concept match returns the right corpus payload (deterministic, graph path)', () => {
  const r = recall('kernel law');
  assert.ok(r.hits.some((h) => h.text.includes('decide/fold/replay is pure')),
    `exact concept should surface the kernel-law payload, got: ${JSON.stringify(r.hits)}`);
});

test('RED: gibberish (no overlap with corpus concepts) returns NO confident hits — recall does not hallucinate', () => {
  // query chosen with zero substring overlap with the seeded corpus concepts (kernel/guard/mesh/...),
  // so graph recall finds nothing and the weak vector fallback (floor 0.85) excludes noise.
  const r = recall('qwfpzm vbnm lkjh tzc');
  assert.equal(r.hits.length, 0, `gibberish must produce no hits, got: ${JSON.stringify(r.hits)}`);
});

test('RED: gibberish must never surface a REAL corpus payload as a confident association', () => {
  const r = recall('zzxqwv nonsense token qwkplm'); // contains "x" → may graph-match the stray "x" node,
  // but must NEVER surface the meaningful seeded payloads (kernel law, guard, mesh, etc.)
  const meaningful = r.hits.filter((h) =>
    /decide\/fold|guard|mesh|kernel|hypervector|SyncPort/i.test(h.text));
  assert.equal(meaningful.length, 0,
    `gibberish must not surface meaningful corpus payloads, got: ${JSON.stringify(r.hits)}`);
});

test('GREEN: recall degrades honestly when §0·GP retriever absent (no spawn, no fabricated note)', () => {
  const r = recall('guard os');
  assert.ok(r.note.includes('not bundled'), `note should say not bundled, got: ${r.note}`);
  assert.ok(!r.note.includes('/root/spikes'), `note must not reference a wrong repo path, got: ${r.note}`);
  assert.ok(!r.note.includes('Command failed'), `note must not show a spawn failure, got: ${r.note}`);
});

test('GREEN: estimateTokens returns null when VSA cli absent (no spawn)', () => {
  assert.equal(estimateTokens('hello world tokens'), null);
});
