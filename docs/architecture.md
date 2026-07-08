# Architecture

Bebop is built as **layers**, each testable in isolation. The invariant: the *core* is pure and
deterministic; the *shell* (CLI, backends, network) is the only place IO lives.

```
            ┌─────────────────────────────────────────────┐
  CLI →     │  bebop.ts  (dispatch; reads env, NEVER files) │
            └───────────────┬─────────────────────────────┘
                            │  EVERY command passes through ↓
            ┌─────────────────────────────────────────────┐
  GUARD OS  │  guard.ts  red-line + scope + certify (pure)  │  ← fail-closed gate
            └───────────────┬─────────────────────────────┘
                            │
        ┌───────────────┬───┴────────┬────────────────┐
        ↓               ↓            ↓                ↓
  kernel.ts        governor.ts   memory.ts        loop.ts
  decide/fold/     PID + ICIR +   VSA insert/      routing +
  replay +         resonance      forget/recall    backend exec
  Checker gate     (autonomy $)   (living memory)  (token ledger)
        │               │            │                │
        ↓               ↓            ↓                ↓
  store.ts        crypto.ts     torrent.ts        mesh.ts
  hash-chained    PQ identity   content-addressed mesh transport
  append-only     + vault       pieces            (swap-not-rewrite)
  log
```

## The determinism contract

`kernel.ts`, `guard.ts`, `governor.ts`, `memory.ts`, `torrent.ts`, `store.ts`, `crypto.ts`
import **only** `node:*` and `@noble/*`. No `Date.now()`, no `Math.random()`, no `fetch`, no
`process.env` *inside* the decision path. The shell supplies nonces/timestamps as arguments.
This is what makes the log replayable, the gate testable, and the whole thing falsifiable.

## "As above, so below"

The same `Checker` abstraction that gates a command locally (`kernel.applyCommandChecked`)
is the invariant a *receiving mesh node* reuses to admit/reject a gossiped envelope. One rule
at two scales — local doer, mesh checker. A violating transition is quarantined into `DENIED`,
never silently admitted.

## Autonomy is a control loop

`governor.ts` is a PID controller over "quality". Authority = controller output, clamped, with
integral anti-windup. Each backend/model is a "factor" scored by ICIR (stability of its
predictions). Before any dynamic change, `loopResonance()` predicts the damping ratio ζ; if
ζ < 0.707 the change is refused. Autonomy can shrink but is engineered never to blow up.

## No central server

`torrent.ts` splits payloads into SHA-256 content-addressed pieces; `mesh.ts` moves pieces
between nodes by hash and verifies every one. Ordering/dedup is the kernel's job (via `cause`),
so the transport only needs to move bytes — libp2p later is a swap, not a rewrite.

## State you can read

Everything Bebop knows lives in files you can open: the hash-chained event log (`store.ts`),
the living memory JSONL (`memory.ts`), and the encrypted identity vault (`vault.ts`). No
opaque cloud. You own the ship.
