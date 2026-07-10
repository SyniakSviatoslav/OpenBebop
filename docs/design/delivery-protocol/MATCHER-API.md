# Matcher API — Open, Replicable Dispatch (kills DANGER #1)

> Status: design + implementation draft (2026-07-10). Counterpart to
> `PROTOCOL-CENTRALIZATION-MAP.md`, which names the **matching/dispatch
> sequencer** as the single most likely hidden-centralization point. This doc
> specifies the OPEN contract that prevents it: the matcher is a **pure,
> deterministic, replicable function** — not a server.

## The problem (DANGER #1 recap)

Whoever decides "which courier serves which order, at what price, in what
sequence" controls the network's economics even if settlement is on-chain. A
single hosted matcher = a silent central bank of dispatch. DoorDash-with-extra-
steps.

## The fix: open + replicable, not "a better server"

The matcher is specified as a **function**, not a service:

```
MatcherRequest  →  MatcherResponse          (deterministic; no hidden state)
fingerprint(resp)                      (content hash; proves node agreement)
```

Because the algorithm is open and the result reproducible + fingerprintable,
any node can serve any request identically. No box is privileged. Swapping the
implementation (local / remote / another vendor) is a client-side choice, not a
protocol change.

## Transport-agnostic contract

The contract is **JSON** (serializable request/response). It rides over ANY
transport — stdio (like `mcp.rs`), HTTP, p2p/gossip, a queue. No proprietary
encoding, no lock-in.

### `MatcherRequest`
```json
{
  "nodes":  [ { "id": "courier0", "x": 0.0, "y": 0.0, "red_line": false }, ... ],
  "edges":  [ { "from": 0, "to": 1, "kind": "Relation", "weight": 1.0 }, ... ],
  "costs":  [ { "latency": 1.0, "cost": 0.0, "risk": 0.0 }, ... ],   // len(edges)
  "orders": [ { "id": "o1", "src": 0, "dst": 2 }, ... ],
  "radius": 200.0
}
```
- `nodes` / `edges` / `costs` describe the live connection graph (see
  `wavefield::Node2D`, `ConnEdge`, `cost_estimate::EdgeCost`).
- `orders[i]`: courier at `src` must reach `dst`.
- `radius`: Layer-1 spatial pre-filter radius (far noise cull).

### `MatcherResponse`
```json
{
  "assignments": [
    { "order_id": "o1", "courier": 0, "path": [0,1,2], "cost": 2.0 }
  ],
  "unmatched": []                 // orders REFUSED (fail-closed), surfaced
}
```
- Each `assignment` = cheapest-adequate route via the Hybrid Cost-Aware Engine
  (k-d filter + BFS guard + A*/Contraction-Hierarchy), i.e. `cost_estimate::
  hybrid_route`.
- `unmatched` = orders the matcher **refused** (unreachable / outside radius).
  They are NOT silently dropped — the caller sees them and may re-dispatch or
  contest. Fail-closed by design.

### `fingerprint(resp) -> u64`
Deterministic content hash of the response (canonical, sorted). Two independent
nodes running the same `MatcherRequest` MUST return the same fingerprint. This
is the verifiable anti-centralization guarantee: agreement without a trusted
server.

## Reference client

`crates/bebop/src/matcher.rs` defines:

- `trait MatcherClient { fn match_batch(&self, &MatcherRequest) -> MatcherResponse; }`
  — the contract callers depend on. Implementation is swappable.
- `LocalMatcherClient` — runs `match_orders` **in-process**. The default: proves
  the matcher needs NO server at all.
- A `RemoteMatcherClient` (HTTP/p2p/stdio) is a thin wrapper that serializes the
  request, sends it over the transport, parses the response. It implements the
  **same trait**, so callers are agnostic to where matching happens.

### Why this kills DANGER #1
1. **No privileged state.** The matcher holds nothing between calls; output is a
   pure function of input.
2. **Replicable.** Anyone can run it; fingerprints agree. No "source of truth".
3. **Replaceable.** The client codes to a trait, not a hostname. A bad/coerced
   matcher is bypassed by pointing the client elsewhere.
4. **Fail-closed + verifiable.** Refusals are explicit; results are fingerprinted
   and contestable before settlement.

## RED+GREEN proofs (in `matcher.rs`)
- `matches_reachable_order` — GREEN: courier reaches destination via correct path.
- `refuses_unreachable_order_fail_closed` — RED+GREEN: unreachable order refused,
  surfaced in `unmatched` (not dropped).
- `matcher_is_replicable_no_hidden_server` — RED+GREEN (the DANGER #1 killer):
  two independent `LocalMatcherClient` instances on the same request produce
  identical fingerprints — the matcher is a pure function, not a server.
- `contract_is_serializable_open_transport` — GREEN: request/response round-trip
  through JSON ⇒ the contract is open over any transport.

## Settlement note
The matcher returns *intent* (who serves whom, at what cost). Settlement is a
separate concern, anchored on the DLT only after the physical Proof-of-Delivery
handoff (the weakest link per the centralization map — needs hardware attestation
bebop cannot fully supply). The matcher never touches money; it proposes.
