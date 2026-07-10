// RISC Zero zkVM integration — RED+GREEN (Verified-by-Math).
//
// What this proves WITHOUT a prover:
//   GREEN  (good):  decide() is deterministic — same input twice yields byte-identical
//                   output; the verifier accepts a correctly-bound journal.
//   RED    (bad):   a tampered journal (one flipped byte) or mismatched input is
//                   rejected (verify returns false).
//
// Prover status: the RISC Zero toolchain install (`curl https://risczero.com/install | bash`)
// was blocked in this environment, so NO real receipt was generated. We do not fabricate
// one. The native TS decide() and the verifier stub together establish the binding
// property the circuit guarantees; a real receipt's STARK proof is delegated to rzup.
//
// Additionally we run the guest crate's own `cargo test` (on the host target, plain
// arithmetic — no risc0 target required) to confirm the Rust decide() is deterministic,
// iff cargo + the source are available.

import assert from 'node:assert/strict';
import test from 'node:test';
import { execFileSync } from 'node:child_process';
import { existsSync } from 'node:fs';
import {
  decide,
  buildJournal,
  toBytes,
} from './decide.ts';
import { verifyJournal, parseJournal } from './verify.ts';

const SAMPLE = {
  state: toBytes('state-0001'),
  cmd: toBytes('allow'),
  ctx: toBytes('epoch-7'),
  counter: 1,
};

test('GREEN: decide() is deterministic (byte-identical across runs)', () => {
  const a = decide(SAMPLE.state, SAMPLE.cmd, SAMPLE.ctx, SAMPLE.counter);
  const b = decide(SAMPLE.state, SAMPLE.cmd, SAMPLE.ctx, SAMPLE.counter);
  assert.equal(a.length, 32, 'must be a 32-byte digest');
  assert.ok(bytesEqual(a, b), 'identical inputs must yield identical output');
});

test('GREEN: decide() is a pure function of its inputs (different input -> different digest)', () => {
  const a = decide(SAMPLE.state, SAMPLE.cmd, SAMPLE.ctx, SAMPLE.counter);
  const b = decide(SAMPLE.state, SAMPLE.cmd, SAMPLE.ctx, SAMPLE.counter + 1);
  const c = decide(toBytes('STATE-0001'), SAMPLE.cmd, SAMPLE.ctx, SAMPLE.counter);
  assert.ok(!bytesEqual(a, b), 'counter change must change digest');
  assert.ok(!bytesEqual(a, c), 'state change must change digest');
});

test('GREEN: verifier accepts a correctly-bound journal', () => {
  const journal = buildJournal(SAMPLE.state, SAMPLE.cmd, SAMPLE.ctx, SAMPLE.counter);
  const res = verifyJournal(journal, {
    state: SAMPLE.state,
    cmd: SAMPLE.cmd,
    ctx: SAMPLE.ctx,
    counter: SAMPLE.counter,
  });
  assert.equal(res.valid, true, `expected valid, got: ${res.reason}`);
});

test('GREEN: verifier re-derives inputs from journal and binds digest', () => {
  const journal = buildJournal(SAMPLE.state, SAMPLE.cmd, SAMPLE.ctx, SAMPLE.counter);
  const parsed = parseJournal(journal);
  assert.ok(bytesEqual(parsed.state, SAMPLE.state));
  assert.equal(parsed.counter, SAMPLE.counter);
  const res = verifyJournal(journal); // no expectedInputs -> self-binding check
  assert.equal(res.valid, true);
});

test('RED: a tampered journal (one flipped byte) fails verification', () => {
  const journal = buildJournal(SAMPLE.state, SAMPLE.cmd, SAMPLE.ctx, SAMPLE.counter);
  // Flip the LAST payload byte (in the ctx region), keeping length fields intact.
  const tampered = new Uint8Array(journal);
  const last = tampered.length - 1;
  tampered[last] ^= 0xff;
  const res = verifyJournal(tampered);
  assert.equal(res.valid, false, 'tampered journal must be rejected');
  assert.match(res.reason ?? '', /tampered/);
});

test('RED: tampering the embedded digest byte fails verification', () => {
  const journal = buildJournal(SAMPLE.state, SAMPLE.cmd, SAMPLE.ctx, SAMPLE.counter);
  const tampered = new Uint8Array(journal);
  tampered[0] ^= 0xff; // flip first digest byte
  const res = verifyJournal(tampered);
  assert.equal(res.valid, false, 'digest tamper must be rejected');
});

test('RED: mismatched claimed input fails verification', () => {
  const journal = buildJournal(SAMPLE.state, SAMPLE.cmd, SAMPLE.ctx, SAMPLE.counter);
  const res = verifyJournal(journal, {
    state: toBytes('WRONG-STATE'),
    cmd: SAMPLE.cmd,
    ctx: SAMPLE.ctx,
    counter: SAMPLE.counter,
  });
  assert.equal(res.valid, false, 'mismatched input must be rejected');
  assert.match(res.reason ?? '', /input mismatch/);
});

test('RED: counter mismatch fails verification', () => {
  const journal = buildJournal(SAMPLE.state, SAMPLE.cmd, SAMPLE.ctx, SAMPLE.counter);
  const res = verifyJournal(journal, {
    state: SAMPLE.state,
    cmd: SAMPLE.cmd,
    ctx: SAMPLE.ctx,
    counter: SAMPLE.counter + 999,
  });
  assert.equal(res.valid, false);
  assert.match(res.reason ?? '', /counter mismatch/);
});

// Optional: confirm the Rust guest's decide() is deterministic via `cargo test`,
// run on the HOST target (plain arithmetic, no risc0 target needed). Skipped if
// cargo or the crate is unavailable.
const cargoAvailable = (() => {
  try {
    execFileSync('cargo', ['--version'], { stdio: 'pipe' });
    return true;
  } catch {
    return false;
  }
})();
const guestExists = existsSync(new URL('./guest/Cargo.toml', import.meta.url));

test(
  'GREEN: Rust guest decide() is deterministic (cargo test, host target)',
  { skip: cargoAvailable && guestExists ? false : 'cargo or guest crate unavailable' },
  () => {
    const out = execFileSync(
      'cargo',
      ['test', '--manifest-path', new URL('./guest/Cargo.toml', import.meta.url).pathname],
      { encoding: 'utf8' },
    );
    assert.match(out, /deterministic/, 'cargo test should report the deterministic test');
  },
);

function bytesEqual(a: Uint8Array, b: Uint8Array): boolean {
  if (a.length !== b.length) return false;
  for (let i = 0; i < a.length; i++) if (a[i] !== b[i]) return false;
  return true;
}
