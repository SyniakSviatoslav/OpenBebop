# tigerbeetle — reverse-engineering notes

## Source (reverse-engineered)
- **TigerBeetle** (Zig, distributed ledger, 1M TPS/box, zero dynamic alloc, idempotent transfers by
  `id`, strict double-entry). We did NOT run a TigerBeetle cluster inside the node (heavy). We ported
  its INVARIANTS into a deterministic in-process `Ledger` (pre-reserved store, double-entry, idempotent
  by transfer id, conservation `Σbalance == 0`). A real `tb_client` drops in behind `Ledger`.

## Invariants captured (the parts that matter for bebop)
1. **Double-entry**: every transfer moves `amount` from `debit` to `credit`; `debit != credit`.
2. **Idempotent by transfer id**: replaying `id` is a no-op (exactly-once money movement).
3. **Conservation law**: `Σ(credit − debit) == 0` across all accounts — money is neither created nor
   destroyed. This is the SAME money boundary the kernel's `applyCommandChecked` gate needs.

## Wiring (max-EV)
- `kernel-ledger.ts`: `applyMoneyTransfer(ledger, t)` applies a money motion through the TigerBeetle
  invariants at SHELL apply-time (identity/money is OUT OF the kernel per kernel.ts). `moneyTransferChecker()`
  is a structural kernel `Checker` for money-tagged Commands (string-encoded bigints, TigerBeetle wire
  format). `moneyConserved()` asserts post-apply conservation — fail-closed, like the kernel.

## Verified-by-Math
- `ledger.test.ts`: 6 GREEN/RED (double-entry, idempotency, conservation, reject unknown account, reject
  debit==credit).
- `kernel-ledger.test.ts`: GREEN valid transfer conserves; RED rejects amount<=0 / debit==credit / ill-formed.
