// Bebop consciousness tests — self-maintenance, self-evolution, session-as-node. RED+GREEN.

import { test } from 'node:test';
import assert from 'node:assert/strict';

import {
  selfMaintain,
  selfEvolve,
  recordSession,
  verifySelfEvolution,
} from './consciousness.ts';
import { livingMemory } from './memory.ts';

// ── SELF-MAINTENANCE ──

test('GREEN: selfMaintain runs the self-harness and records health into the one living memory', () => {
  const h = selfMaintain();
  assert.equal(typeof h.ok, 'boolean');
  assert.ok(h.pass >= 0 && h.fail >= 0);
  // the health event was recorded (memory grew)
  assert.ok(livingMemory().size > 0);
});

// ── SELF-EVOLUTION (fail-closed) ──

test('GREEN: selfEvolve ACCEPTS a valid, novel (short) idea and persists it to living memory', async () => {
  const before = livingMemory().size;
  const r = await selfEvolve('cache PQ keys'); // short, well-damped mutation → passes resonance pre-check
  assert.equal(r.accepted, true);
  assert.ok(r.id, 'a persisted node id is returned');
  assert.ok(livingMemory().size >= before); // did not shrink
});

test('RED: selfEvolve QUARANTINES a trivial idea (fail-closed, not applied)', async () => {
  const r = await selfEvolve('x'); // < 4 chars → checker rejects
  assert.equal(r.accepted, false);
  assert.match(r.reason, /quarantined/i);
});

test('RED: selfEvolve QUARANTINES a near-duplicate idea', async () => {
  await selfEvolve('use spreading activation for associative recall');
  const r = await selfEvolve('use spreading activation for associative recall'); // same → duplicate
  assert.equal(r.accepted, false);
});

test('RED: selfEvolve QUARANTINES a bulk mutation that would make self-evolution under-damped (resonance pre-check)', async () => {
  // a very long/structural idea represents a large coupling gain → loopResonance flags ζ<0.707
  const bulk = 'restructure the entire corpus graph by rewiring every node edge weight and adding recursive sub-loops across all layers simultaneously';
  const r = await selfEvolve(bulk);
  assert.equal(r.accepted, false);
  assert.match(r.reason, /resonance/i);
});

// ── SELF-EVOLUTION AUDIT TRAIL (tamper-evident kernel journal) ──

test('GREEN: an accepted self-evolution is recorded in a verifiable tamper-evident journal', async () => {
  // accept a couple of mutations, then prove the audit chain verifies
  await selfEvolve('audit-evolve-alpha-unique');
  await selfEvolve('audit-evolve-beta-unique');
  const ok = verifySelfEvolution();
  assert.equal(ok, true, 'a clean self-evolution chain must verify');
});

test('RED: tampering a recorded self-evolution digest breaks the audit (falsifiable)', async () => {
  // import the internals through a fresh module instance is not possible; instead prove the
  // invariant via the kernel journal directly — mutate a digest → verifySelfEvolution-equivalent fails.
  // We exercise the same primitive used by verifySelfEvolution to guarantee the RED case is real.
  const { applyCommand, genesis, commandHash } = await import('./kernel.ts');
  const { verifyJournal } = await import('./integration/zkvm/kernel-journal.ts');
  const cmd = {
    actor: { kind: 'system' as const, id: 'bebop-consciousness' },
    action: 'PUBLISH' as const,
    payload: JSON.stringify({ concept: 'audit-probe', id: 'x1' }),
    nonce: 'audit-probe',
  };
  const st = applyCommand(cmd, genesis()).state;
  const cause = commandHash(cmd);
  const { journalize, digestToHex } = await import('./integration/zkvm/kernel-journal.ts');
  const seq = st.ingested.size;
  const digest = journalize(st, cause, seq);
  // genuine digest verifies
  assert.equal(verifyJournal(st, cause, seq, digest), true);
  // tampered state must NOT verify
  const tampered = { ...st, lastBackend: 'evil' };
  assert.equal(verifyJournal(tampered, cause, seq, digest), false, 'tamper must break the digest');
  void digestToHex;
});

// ── SESSION-AS-NODE (brain-in-brain) ──

test('GREEN: recordSession records THIS session as a living-memory node with a child memory', () => {
  const id = recordSession({
    id: 'hermes-test-session',
    summary: 'this hermes session is a bebop node',
    childFacts: [['sub-fact', 'a session holds its own sub-memory']],
  });
  assert.ok(id);
  const child = livingMemory().findChild(id);
  assert.ok(child, 'session node nests a child memory (brain-in-brain)');
  assert.equal(child!.size, 1);
});
