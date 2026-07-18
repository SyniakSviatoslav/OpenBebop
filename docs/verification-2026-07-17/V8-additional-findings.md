# V8 — Additional findings beyond the known 7 (fresh re-audit)

These surfaced during the fresh re-audit and were **not** in the starting checklist.

---

## V8.1 — Replay-ledger eviction reopens the replay window (MEDIUM)

**File:** `bebop2/proto-cap/src/hybrid_gate.rs:56, 193-206` (cited @ `b87b7e2`)

`HybridGate` defends against replay with an in-process `seen: Mutex<HashSet<[u8;8]>>` of accepted
nonces. To avoid unbounded memory growth (red-team B2/B3 DoS), the set is capped at
`MAX_SEEN_NONCES = 1 << 20` (~1M) and pruned when exceeded:

```rust
// hybrid_gate.rs:201-205
if seen.len() > MAX_SEEN_NONCES {
    let keep: HashSet<[u8; 8]> =
        seen.iter().take(MAX_SEEN_NONCES / 2).copied().collect();  // drop ~half
    *seen = keep;
}
```

The comment (`:200`) asserts: *"Order is irrelevant for replay defense — any half is fine."*
That is **incorrect**. Evicting a nonce from `seen` means a later frame carrying that **same**
nonce will `insert` successfully (return `true`) and be treated as **fresh**, i.e. accepted
again. Eviction therefore *reopens* the replay window for every evicted nonce.

**Exploit shape:** an authorized peer (one that can legitimately get frames accepted) submits
> 1M distinct-nonce frames to force a prune, then **replays** a previously-accepted frame whose
nonce landed in the evicted half. The gate, having forgotten that nonce, re-accepts the replay.
The signature/chain/expiry all still verify (it was a genuine frame), so nothing else stops it.

**Why MEDIUM, not HIGH:** the attacker must already be authorized, must push the ledger past 1M
entries, and can only replay frames whose nonces happen to be evicted (a random ~half). Nonces
are `[u8;8]`; expiry (`is_fresh(now)`) still bounds replay to the capability's unexpired window.
But the invariant "an accepted nonce is never accepted twice" is silently violated under memory
pressure, which is precisely when a DoS-minded attacker operates.

**Remediation:** don't rely on an evictable set for replay defense. Use a monotonic
per-issuer/per-subject nonce **counter** (reject `nonce <= high_water`) or a sliding-window
scheme tied to expiry (evict only nonces whose capability has already expired — those can never
be validly replayed anyway). If a bounded set is kept, evict by **expiry**, never arbitrarily.

---

## V8.2 — Plans-vs-implementation ledger (cross-cutting)

The charter asks specifically for correspondence to **what is claimed in plans**, not just to the
implementation. Consolidated from V1–V7:

| Claimed in plans / docs | Implementation reality (b87b7e2) | Ref |
|---|---|---|
| Revocation "closes the biggest authz hole" mesh-wide via gossip convergence | Local set is sound; **gossip/merge accepts unsigned revocations** → converges on whatever any peer asserts | V1 |
| Red-line brake on validly-signed money/settlement mutations (blueprint G5) | Brake exists + tested, but the **only prod seam (`KernelFacade`) never arms it** and offers no API to | V2 |
| "Fail-closed production keygen from real entropy" | Entropy + fail-closed are correct, but the returned **secret key is a copy of the public key**; the real seed is dropped | V3 |
| Capability-based (not reputation) **Sybil resistance** with bounded authority | Attenuation lattice enforced rigorously; **issuance quantity + chain depth are unbounded** → Sybil bound is a policy assumption, plus a verify-cost DoS | V4 |
| Operator-selectable root-delegation models (OperatorSigned/WoT/QR) | Only "chosen vs unspecified" is enforced; the **three real models are never dispatched**, no rate-limit | V5 |
| CI gate double-locks NO-COURIER-SCORING | Gate is porous to `<prefix>_score` / `trust_weight` compounds (`\b`+underscore) | V6 |
| Sovereign, local-first, **runs-everywhere** mesh (mobile courier use-case) | Core **does not compile for Android/iOS** (`compile_error!`) | V7 |

Pattern: the **cryptographic lattice / state-machine logic is consistently strong and honestly
documented**, while the **wiring, propagation-authentication, resource bounds, and portability**
lag the claims. The recurring gap is "the safe primitive exists and is unit-tested, but the
production path either doesn't reach it (V2), doesn't bound it (V4, V8.1), doesn't authenticate
its propagation (V1), or doesn't compile where the plan says it runs (V7)."

---

## V8.3 — Notes checked and found NOT to reproduce / not findings

- **`verify()` malleability** — `sign.rs:841-844` correctly rejects non-canonical `S ≥ L` before
  point ops. No malleability finding. (No `verify_batch` on this branch, as expected — the
  SSR-2020 batch fix is on `feat/b4-crypto-groundtruth-bench`, not here.)
- **`claim_machine.rs` scoring** — the state machine itself is clean (no score field). The V6
  finding is against the CI regex, not the machine.
- **`RootDelegationPolicy::Unspecified` fail-closed default** — this is *correct* behavior
  (`require_explicit_policy` rejects it); the V5 finding is narrowed to the three *real* variants
  being indistinguishable, not to the default being wrong.
- **`compile_error!` on unsupported targets (V7)** — the *fail-closed itself* is correct; the
  finding is the plans-vs-impl portability gap, not unsafe fallback behavior.
