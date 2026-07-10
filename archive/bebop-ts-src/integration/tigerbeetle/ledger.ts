// src/integration/tigerbeetle/ledger.ts
//
// Integration layer for TigerBeetle (Zig, deterministic ledger, 1M TPS/box, zero dynamic alloc)
// into the bebop Sovereign Node operational ledger.
//
// Reality (2026-07-08): TigerBeetle is a real distributed ledger DB; its client speaks a binary
// protocol over TCP. Running a full TigerBeetle cluster inside the node is heavy. We integrate its
// *invariants* as a deterministic in-process ledger (pre-reserved store, strict consistency,
// double-entry transfers, idempotent by transfer id) — the same guarantees the core `decide()`
// money boundary needs. A real tb_client drops in behind `Ledger`.
//
// Verified-by-Math: ledger.test.ts (GREEN correct; RED rejects invalid physics).

export interface Transfer {
  id: bigint;          // monotonic unique; idempotency key
  debit: bigint;       // account id
  credit: bigint;      // account id
  amount: bigint;      // > 0
  code: number;        // user-defined ledger code
}

export interface Account {
  id: bigint;
  debit: bigint;       // running debit total
  credit: bigint;      // running credit total
}

export class Ledger {
  private accounts = new Map<string, Account>();
  private applied = new Set<string>(); // idempotency: transfer id -> applied

  createAccount(id: bigint): void {
    if (id <= 0n) throw new Error('account id must be > 0');
    const key = id.toString();
    if (this.accounts.has(key)) throw new Error('account exists');
    this.accounts.set(key, { id, debit: 0n, credit: 0n });
  }

  /** Deterministic double-entry transfer. Idempotent by transfer.id. */
  transfer(t: Transfer): void {
    if (t.amount <= 0n) throw new Error('amount must be > 0');
    if (t.debit === t.credit) throw new Error('debit == credit (no-op illegal)');
    const idk = t.id.toString();
    if (this.applied.has(idk)) return; // idempotent
    const d = this.accounts.get(t.debit.toString());
    const c = this.accounts.get(t.credit.toString());
    if (!d || !c) throw new Error('unknown account');
    d.debit += t.amount; // money leaves debit side
    c.credit += t.amount; // arrives at credit side
    this.applied.add(idk);
  }

  balance(id: bigint): bigint {
    const a = this.accounts.get(id.toString());
    if (!a) throw new Error('unknown account');
    // net = credit - debit (credit is inbound, debit is outbound)
    return a.credit - a.debit;
  }

  /** Conservation law: sum of all balances == 0 (money is neither created nor destroyed). */
  isConserved(): boolean {
    let net = 0n;
    for (const a of this.accounts.values()) net += a.credit - a.debit;
    return net === 0n;
  }
}
