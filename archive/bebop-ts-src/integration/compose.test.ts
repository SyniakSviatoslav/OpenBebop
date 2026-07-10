// src/integration/compose.test.ts
//
// RED+GREEN: the assembled Sovereign Node composes zkVM journal + TigerBeetle money boundary +
// Active Inference advisor around the PURE kernel (kernel.ts is untouched).

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { applyWithJournal, verifyJournalChain, advise, type JournalEntry } from './compose.ts';
import { genesis, commandHash, type Command, type State } from '../kernel.ts';

function cmd(action: Command['action'], payload: string, money = false): Command {
  return {
    actor: { kind: 'node', id: 'self' },
    action,
    payload: money ? JSON.stringify({ debit: '1', credit: '2', amount: '50', id: '1' }) : payload,
    nonce: 'n' + payload,
  };
}

test('GREEN: applyWithJournal admits a normal command and records a digest', () => {
  let st: State = genesis();
  const r = applyWithJournal(cmd('INGEST', 'abc'), st);
  assert.ok(!r.quarantined);
  assert.equal(r.state.ingested.has('abc'), true);
  assert.equal(r.journal.digest.length, 32, 'zkVM digest must be 32 bytes');
  assert.equal(r.journal.cause, commandHash(cmd('INGEST', 'abc')));
  st = r.state;
});

test('GREEN: journal chain verifies against the final state', () => {
  let st: State = genesis();
  const cmds: Command[] = [cmd('INGEST', 'a'), cmd('PUBLISH', 'b'), cmd('INGEST', 'c')];
  const journals: JournalEntry[] = [];
  for (const c of cmds) {
    const r = applyWithJournal(c, st);
    assert.ok(!r.quarantined);
    st = r.state;
    journals.push(r.journal);
  }
  assert.ok(verifyJournalChain(cmds, journals, st), 'journal chain must verify');
});

test('RED: a money-tagged command with debit==credit is quarantined by the TigerBeetle boundary', () => {
  const bad: Command = {
    actor: { kind: 'node', id: 'self' },
    action: 'DISPATCH',
    payload: JSON.stringify({ debit: '1', credit: '1', amount: '50', id: '1' }),
    nonce: 'bad',
  };
  const r = applyWithJournal(bad, genesis(), { money: true });
  assert.ok(r.quarantined, 'debit==credit must be quarantined');
  assert.match(r.reason ?? '', /debit == credit/);
});

test('GREEN: a valid money-tagged command passes the boundary (structural only; conservation at apply-time)', () => {
  const ok: Command = {
    actor: { kind: 'node', id: 'self' },
    action: 'DISPATCH',
    payload: JSON.stringify({ debit: '1', credit: '2', amount: '50', id: '1' }),
    nonce: 'ok',
  };
  const r = applyWithJournal(ok, genesis(), { money: true });
  assert.ok(!r.quarantined, 'valid money motion must pass the structural check');
});

test('GREEN: Active Inference advisor picks done from a done-belief', () => {
  assert.equal(advise([0, 0, 1]), 'done');
});

test('RED: tampering with a journal digest fails the chain (falsifiable)', () => {
  let st: State = genesis();
  const cmds: Command[] = [cmd('INGEST', 'a'), cmd('INGEST', 'b')];
  const journals: JournalEntry[] = [];
  for (const c of cmds) {
    const r = applyWithJournal(c, st);
    st = r.state;
    journals.push(r.journal);
  }
  journals[0].digest[0] ^= 0xff; // flip one byte
  assert.ok(!verifyJournalChain(cmds, journals, st), 'tampered digest must fail the chain');
});
