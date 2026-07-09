# zenoh — reverse-engineering notes

## Source (reverse-engineered)
- **Eclipse Zenoh** (`@eclipse-zenoh/zenoh-ts@1.9.0`, Rust core + TS bindings) — pub/sub + queryable
  + decentralized peer mesh (scout/gossip). We did NOT pull the native binary (offline/constrained
  node; native client would add a flaky Rust toolchain dependency). Instead we ported Zenoh's
  CORE SEMANTICS into a deterministic in-process `ZenohTransport` with the same interface, so a real
  `createZenoh()` is a drop-in swap behind the type.

## Semantics captured (the parts that matter for bebop)
1. **Named-key pub/sub** — `put(env)` publishes on a key expression; subscribers to a prefix receive.
2. **Decentralized mesh** — multiple local nodes form a gossip mesh; `put` returns delivery count.
3. **Priority arbitration (CAN-bus style)** — lower `priority` id wins non-destructively; ties broken
   by `from` id. Deterministic, no clock.
4. **Last-value store + query** — `get(key)` returns the latest put on that key (the `store` map).

## Where it wires in (max-EV)
- **Sovereign Node telemetry fan-out**: L5 governor emits `l5/telemetry/<node>`; peers subscribe.
  Priority arbitration keeps control-plane msgs ahead of bulk telemetry.
- **Drop-in for the mesh transport** in `loop.ts` inter-node comms (currently single-node).

## Verified-by-Math
- `transport.test.ts`: 6 GREEN/RED (publish→subscribe, mesh gossip count, priority wins, store/get,
  rejects negative priority, rejects non-monotonic seq).
