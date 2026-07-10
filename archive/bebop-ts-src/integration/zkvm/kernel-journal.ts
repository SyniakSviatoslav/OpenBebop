// src/integration/zkvm/kernel-journal.ts
//
// WIRING: RISC Zero zkVM `decide()` as a tamper-evident journal over the bebop kernel.
//
// Per kernel.ts, identity/signing is OUT OF THE KERNEL (shell envelope); the kernel only produces a
// content-addressed `commandHash`. This module feeds (state bytes + command hash + monotonic counter)
// into the deterministic zkVM `decide()` to produce a journal digest that is tamper-evident: flipping
// any logged byte changes the digest (proven by zkvm.test.ts). That gives the kernel a verifiable
// content journal without the kernel itself signing anything.
//
// Honest scope (see NOTES.md): this runs the NATIVE TS port of decide(); actual STARK proving needs
// the risc0 toolchain/prover (blocked in this env). The digest is deterministic + tamper-evident; the
// cryptographic *receipt* is best-effort and documented as such.

import { decide } from './decide.ts';

/** Serialize kernel state to canonical bytes (order-independent via sorted set hashes). */
export function serializeState(state: {
  ingested: Set<string>;
  published: Set<string>;
  revoked: Set<string>;
  seen: Set<string>;
  lastBackend: string | null;
}): Uint8Array {
  const parts = [
    [...state.ingested].sort().join(','),
    [...state.published].sort().join(','),
    [...state.revoked].sort().join(','),
    [...state.seen].sort().join(','),
    state.lastBackend ?? '',
  ];
  const s = parts.join('|');
  const out = new Uint8Array(s.length);
  for (let i = 0; i < s.length; i++) out[i] = s.charCodeAt(i) & 0xff;
  return out;
}

/**
 * Produce a tamper-evident journal digest for a kernel transition.
 * `counter` MUST be monotonic (the shell supplies it, NOT RNG); it binds the entry to a sequence so
 * a replayed or reordered entry yields a different digest. Returns the zkVM decide() digest bytes.
 */
export function journalize(state: Parameters<typeof serializeState>[0], commandHash: string, counter: number): Uint8Array {
  const st = serializeState(state);
  const ctx = new TextEncoder().encode(commandHash);
  return decide(st, ctx, new Uint8Array([0]), counter);
}

/** Hex-encode a digest (for embedding in a JOURNAL envelope / logging). */
export function digestToHex(d: Uint8Array): string {
  let s = '';
  for (let i = 0; i < d.length; i++) s += d[i].toString(16).padStart(2, '0');
  return s;
}
export function verifyJournal(
  state: Parameters<typeof serializeState>[0],
  commandHash: string,
  counter: number,
  claimed: Uint8Array,
): boolean {
  const actual = journalize(state, commandHash, counter);
  if (actual.length !== claimed.length) return false;
  for (let i = 0; i < actual.length; i++) if (actual[i] !== claimed[i]) return false;
  return true;
}
