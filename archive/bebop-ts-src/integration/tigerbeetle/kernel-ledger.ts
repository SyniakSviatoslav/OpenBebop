// src/integration/tigerbeetle/kernel-ledger.ts
//
// WIRING: TigerBeetle as the money-conservation boundary for the Sovereign Node.
//
// Per kernel.ts, identity/money is OUT OF THE KERNEL (the kernel only consumes a plain Command +
// State). So the money boundary lives at the SHELL: a transfer is applied through TigerBeetle's
// deterministic `Ledger` (double-entry, idempotent by transfer id, conservation-enforced) BEFORE a
// money command is admitted. This module is that shell seam — the same guarantees a real tb_client
// gives, drop-in behind `Ledger`.
//
// Verified-by-Math: kernel-ledger.test.ts (GREEN correct; RED rejects money that mints/burns).

import { Ledger, type Transfer } from './ledger.ts';
import type { Checker } from '../../kernel.ts';

/** Apply a money transfer through the TigerBeetle invariants. Idempotent + conservation-checked. */
export function applyMoneyTransfer(ledger: Ledger, t: Transfer): void {
  ledger.transfer(t); // throws on amount<=0, debit==credit, unknown account, or non-conserving
}

/** Assert the live ledger still conserves money (Σbalance == 0). Pure read; shell asserts post-apply. */
export function moneyConserved(ledger: Ledger): boolean {
  return ledger.isConserved();
}

/**
 * Structural kernel Checker for a money-tagged command. The kernel State does not hold money, so
 * this validates the *shape* of a money motion carried on the Command (amount > 0, distinct legs)
 * — the real conservation is enforced by `applyMoneyTransfer` at shell apply-time. Pure, no ledger.
 *
 * A Command "carries" a motion by encoding it in `payload` as JSON with STRING-encoded bigints
 * (TigerBeetle's wire format): { debit, credit, amount, id, code } where numeric ids/amounts are
 * strings. This is JSON-serializable (BigInt is not) and BN-parsable at shell apply-time.
 */
export function moneyTransferChecker(): Checker {
  return (cmd) => {
    let motion: Record<string, unknown> = {};
    try {
      motion = JSON.parse(cmd.payload) as Record<string, unknown>;
    } catch {
      return { ok: true }; // not a money command — let the kernel's own checker decide
    }
    if (motion.amount !== undefined || motion.debit !== undefined) {
      // It claims to be a money motion → enforce the structural law.
      // Parse defensively: malformed numeric fields are a QUARANTINE, not an uncaught throw
      // (a throw would fail-open the whole command stream — a red-team DoS/robustness finding).
      let amt: bigint, debit: bigint, credit: bigint;
      try {
        if (typeof motion.amount !== 'string' || typeof motion.debit !== 'string' || typeof motion.credit !== 'string') {
          return { ok: false, reason: 'money motion: debit/credit/amount must be string-encoded bigints' };
        }
        amt = BigInt(motion.amount);
        debit = BigInt(motion.debit);
        credit = BigInt(motion.credit);
      } catch {
        return { ok: false, reason: 'money motion: malformed bigint in debit/credit/amount' };
      }
      if (amt <= 0n) return { ok: false, reason: 'money motion: amount must be > 0' };
      if (debit === credit) {
        return { ok: false, reason: 'money motion: debit == credit (no-op illegal)' };
      }
    }
    return { ok: true };
  };
}
