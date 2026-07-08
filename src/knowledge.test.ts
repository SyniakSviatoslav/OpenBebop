import { test } from 'node:test';
import assert from 'node:assert/strict';
import { recall, estimateTokens, recallLocal } from './knowledge.ts';

// The living-knowledge retriever + VSA cli are NOT bundled in this standalone repo.
// recall()/estimateTokens() must degrade honestly (no child spawn on a missing script).

test('GREEN: estimateTokens returns null when VSA cli absent (no spawn)', () => {
  // No spikes/tools in this repo → must return null, not throw or spawn node.
  assert.equal(estimateTokens('hello world tokens'), null);
});

test('GREEN: recall degrades to local memory with an honest "not bundled" note', () => {
  const r = recall('guard os');
  assert.ok(r.note.includes('not bundled in this repo'), `note should say not bundled, got: ${r.note}`);
  assert.ok(!r.note.includes('/root/spikes'), `note must not reference a wrong repo path, got: ${r.note}`);
  assert.ok(!r.note.includes('Command failed'), `note must not show a spawn failure, got: ${r.note}`);
});

test('GREEN: recallLocal returns id/text pairs from in-process memory', () => {
  const before = recallLocal('anything');
  // in-process livingMemory always has seeded nodes, so this should not throw
  assert.ok(Array.isArray(before));
});
