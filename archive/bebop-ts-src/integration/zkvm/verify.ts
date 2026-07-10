// Verifier stub for a RISC Zero receipt.
//
// In a real deployment this would call `risc0_zkvm::Receipt::verify(IMAGE_ID)` on
// the host, which cryptographically checks the STARK proof and returns the
// authenticated journal. Here (prover-less) we implement the same *integrity
// contract* on the journal that the circuit guarantees:
//
//   - The journal's embedded digest must equal decide(state, cmd, ctx, counter)
//     recomputed from the *claimed* inputs. If an attacker flips a journal byte
//     OR supplies mismatched inputs, the recomputed digest won't match -> false.
//   - A tampered journal (any byte changed) breaks the embedded digest.
//
// NOTE: without a real receipt we cannot check the STARK proof itself. This stub
// proves the *binding* property (journal <-> inputs) that the zkVM enforces; the
// cryptographic soundness of the proof is delegated to the real prover/verifier.

import { decide, buildJournal } from './decide.ts';

export interface Claim {
  state: Uint8Array;
  cmd: Uint8Array;
  ctx: Uint8Array;
  counter: number;
}

/** Parse a journal produced by buildJournal(): digest(32) || counter(4) || lens(12) || payload. */
export function parseJournal(journal: Uint8Array): Claim & { digest: Uint8Array } {
  const digest = journal.subarray(0, 32);
  const counter = (journal[32] | (journal[33] << 8) | (journal[34] << 16) | (journal[35] << 24)) >>> 0;
  const stateLen = (journal[36] | (journal[37] << 8) | (journal[38] << 16) | (journal[39] << 24)) >>> 0;
  const cmdLen = (journal[40] | (journal[41] << 8) | (journal[42] << 16) | (journal[43] << 24)) >>> 0;
  const ctxLen = (journal[44] | (journal[45] << 8) | (journal[46] << 16) | (journal[47] << 24)) >>> 0;
  const state = journal.subarray(48, 48 + stateLen);
  const cmd = journal.subarray(48 + stateLen, 48 + stateLen + cmdLen);
  const ctx = journal.subarray(48 + stateLen + cmdLen, 48 + stateLen + cmdLen + ctxLen);
  return { digest: new Uint8Array(digest), state: new Uint8Array(state), cmd: new Uint8Array(cmd), ctx: new Uint8Array(ctx), counter };
}

function bytesEqual(a: Uint8Array, b: Uint8Array): boolean {
  if (a.length !== b.length) return false;
  for (let i = 0; i < a.length; i++) if (a[i] !== b[i]) return false;
  return true;
}

export interface VerifyResult {
  valid: boolean;
  reason?: string;
}

/**
 * Verify a journal (prover-less binding check).
 * @param journal  the committed journal
 * @param claim    the *claimed* inputs to check binding against (the verifier must
 *                 independently know the expected inputs; here we re-derive them)
 * @param expectedInputs  if provided, the claimed inputs must match these exactly
 */
export function verifyJournal(journal: Uint8Array, expectedInputs?: Claim): VerifyResult {
  if (journal.length < 48) return { valid: false, reason: 'journal too short' };
  const parsed = parseJournal(journal);

  if (expectedInputs) {
    if (parsed.counter !== expectedInputs.counter) {
      return { valid: false, reason: 'counter mismatch' };
    }
    if (
      !bytesEqual(parsed.state, expectedInputs.state) ||
      !bytesEqual(parsed.cmd, expectedInputs.cmd) ||
      !bytesEqual(parsed.ctx, expectedInputs.ctx)
    ) {
      return { valid: false, reason: 'input mismatch' };
    }
  }

  // Recompute the digest from the journal's own embedded inputs and check binding.
  const recomputed = decide(parsed.state, parsed.cmd, parsed.ctx, parsed.counter);
  if (!bytesEqual(recomputed, parsed.digest)) {
    return { valid: false, reason: 'tampered journal: digest does not bind to payload' };
  }
  return { valid: true };
}

/** Simulate a genuine receipt verify(true) result for when a real prover is used. */
export function fakeReceiptVerifyOk(journal: Uint8Array): VerifyResult {
  return verifyJournal(journal);
}

export { buildJournal };
