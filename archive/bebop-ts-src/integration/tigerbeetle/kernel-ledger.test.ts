// src/integration/tigerbeetle/kernel-ledger.test.ts
//
// RED+GREEN: TigerBeetle as the money boundary. GREEN = valid double-entry applies + conserves;
// RED = a transfer that mints/burns money (or is ill-formed) is rejected.

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { Ledger } from './ledger.ts';
import { applyMoneyTransfer, moneyConserved, moneyTransferChecker } from './kernel-ledger.ts';

function seededLedger(): Ledger {
  const l = new Ledger();
  l.createAccount(1n);
  l.createAccount(2n);
  return l;
}

test('GREEN: a valid double-entry transfer applies and conserves money', () => {
  const l = seededLedger();
  applyMoneyTransfer(l, { id: 1n, debit: 1n, credit: 2n, amount: 100n, code: 1 });
  assert.equal(l.balance(1n), -100n);
  assert.equal(l.balance(2n), 100n);
  assert.ok(moneyConserved(l), 'Σbalance must be 0 after a valid transfer');
});

test('GREEN: idempotent transfer does not double-apply', () => {
  const l = seededLedger();
  const t = { id: 5n, debit: 1n, credit: 2n, amount: 50n, code: 1 };
  applyMoneyTransfer(l, t);
  applyMoneyTransfer(l, t); // replay
  assert.equal(l.balance(2n), 50n, 'balance must not double-apply');
});

test('RED: moneyTransferChecker rejects a transfer with amount <= 0', () => {
  const check = moneyTransferChecker();
  const cmd = { actor: { kind: 'node' as const, id: 'x' }, action: 'DISPATCH' as const, payload: JSON.stringify({ debit: '1', credit: '2', amount: '0', id: '1' }), nonce: 'n' };
  const v = check(cmd as any, {} as any, {} as any, []);
  assert.ok(!v.ok, 'amount<=0 must be rejected');
});

test('RED: moneyTransferChecker rejects debit == credit', () => {
  const check = moneyTransferChecker();
  const cmd = { actor: { kind: 'node' as const, id: 'x' }, action: 'DISPATCH' as const, payload: JSON.stringify({ debit: '1', credit: '1', amount: '10', id: '1' }), nonce: 'n' };
  const v = check(cmd as any, {} as any, {} as any, []);
  assert.ok(!v.ok, 'debit==credit must be rejected');
});

test('RED: moneyTransferChecker ignores non-money payloads (lets kernel decide)', () => {
  const check = moneyTransferChecker();
  const cmd = { actor: { kind: 'node' as const, id: 'x' }, action: 'PUBLISH' as const, payload: 'not-json', nonce: 'n' };
  const v = check(cmd as any, {} as any, {} as any, []);
  assert.ok(v.ok, 'non-money payload must pass through');
});
