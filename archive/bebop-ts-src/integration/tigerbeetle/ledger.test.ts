// src/integration/tigerbeetle/ledger.test.ts
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { Ledger } from './ledger.ts';

test('GREEN: double-entry transfer moves money deterministically', () => {
  const l = new Ledger();
  l.createAccount(1n);
  l.createAccount(2n);
  l.transfer({ id: 1n, debit: 1n, credit: 2n, amount: 100n, code: 1 });
  assert.equal(l.balance(1n), -100n);
  assert.equal(l.balance(2n), 100n);
});

test('GREEN: conservation law holds (Σ balance == 0)', () => {
  const l = new Ledger();
  for (const id of [1n, 2n, 3n]) l.createAccount(id);
  l.transfer({ id: 1n, debit: 1n, credit: 2n, amount: 50n, code: 1 });
  l.transfer({ id: 2n, debit: 2n, credit: 3n, amount: 20n, code: 1 });
  assert.ok(l.isConserved());
});

test('GREEN: idempotent by transfer id (replay is a no-op)', () => {
  const l = new Ledger();
  l.createAccount(1n); l.createAccount(2n);
  l.transfer({ id: 5n, debit: 1n, credit: 2n, amount: 10n, code: 1 });
  l.transfer({ id: 5n, debit: 1n, credit: 2n, amount: 999n, code: 1 }); // replay, ignored
  assert.equal(l.balance(2n), 10n);
});

test('RED: amount <= 0 rejected (money boundary invariant)', () => {
  const l = new Ledger();
  l.createAccount(1n); l.createAccount(2n);
  assert.throws(() => l.transfer({ id: 1n, debit: 1n, credit: 2n, amount: 0n, code: 1 }), /amount must be > 0/);
  assert.throws(() => l.transfer({ id: 2n, debit: 1n, credit: 2n, amount: -5n, code: 1 }), /amount must be > 0/);
});

test('RED: debit == credit rejected (self-loop illegal)', () => {
  const l = new Ledger();
  l.createAccount(1n);
  assert.throws(() => l.transfer({ id: 1n, debit: 1n, credit: 1n, amount: 1n, code: 1 }), /debit == credit/);
});

test('RED: transfer to unknown account rejected', () => {
  const l = new Ledger();
  l.createAccount(1n);
  assert.throws(() => l.transfer({ id: 1n, debit: 1n, credit: 99n, amount: 1n, code: 1 }), /unknown account/);
});
