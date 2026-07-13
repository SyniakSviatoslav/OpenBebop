# bebop2 — Protocol Authorization Red-Team Re-Verification (2026-07-13)

**Target:** `/root/bebop-repo/bebop2` @ `feat/logic-governance` — crates `proto-cap` (authorization) + `core` (signatures), plus the `proto-wire` receive path that consumes them.
**Method:** read-only source trace of every kill-chain link from `RED-TEAM-REVIEW-2026-07-12.md §2`, cross-checked against a **compiled, executed PoC** (scratch crate path-depending on the real crates read-only; the target tree was not modified, no git was run). "Claimed fixed" was tested, not trusted.
**PoC location (out-of-tree):** `…/scratchpad/b2pwn/` — `cargo run --release` output reproduced verbatim below.

---

## 1. Bottom line

**The node is STILL CAPTURED.** A random, unenrolled key takes over a node today, exactly as in the 2026-07-12 review. One forged frame — attacker's own key, self-issued capability, lowest-priv scope masking a `ledger.append` drain, PQ leg stripped, expiry defeated — is **accepted by node 1 and replayed verbatim to node 2**.

The remediation blueprint's centrepiece — an `AnchorRoster` + UCAN-subset `verify_chain` root-of-trust — **was written (`proto-cap/src/roster.rs`, 520 LOC, 6 RED/GREEN tests, all green) but never wired into the acceptance path.** It is dead code from the wire's perspective: `HybridGate::check` never calls `verify_chain`, never holds an `AnchorRoster`, and never reads `capability.scope`. This is the repo's signature failure mode — **"claimed green while failing"**: the RED test `red_self_issued_delegation_rejected_as_unknown_issuer` passes, but it exercises `verify_chain` in isolation, a function the node never invokes.

Grep proof (whole tree, excluding `roster.rs` itself and the `lib.rs:43` re-export):
`AnchorRoster` / `verify_chain` / `subject_in_roster` / scope-subset checks appear **NOWHERE** on any acceptance path.

### PoC output (real `cargo run --release` against the live crates)

```
=== bebop2 feat/logic-governance — NODE-TAKEOVER RE-VERIFICATION ===

[*] attacker minted a FRESH keypair (no enrollment): pk=248a..
[*] forged frame: self-issued cap, scope=Presence/Send, payload=ledger.append drain=ALL, expiry=1, NO pq_sig, NO channel_binding

NODE 1  gate.check(&frame, now=0) => Ok(())
        ACCEPTED. self-issue OK, scope unchecked, expiry bypassed (now=0), PQ stripped.

NODE 2  gate.check(SAME frame, now=0) => Ok(())
        ACCEPTED the replayed bytes (per-connection nonce set defeated).

[*] verify_chain(empty_roster, self-delegation, cap) => Err(UnknownIssuer)
    ^ the anchored fix REJECTS the attacker — but HybridGate::check NEVER calls verify_chain,
      never holds an AnchorRoster, and never reads cap.scope. It is dead code on the wire path.

[*] SAME frame with a REAL clock (now=100) => Err(Expired)
    ^ rejected only because now>expiry; the wire path hardcodes now=0 so it never rejects.

=== RESULT: NODE TAKEOVER STILL SUCCEEDS on feat/logic-governance ===
```

`gate.check(&frame, 0)` is byte-for-byte the decision the real receive path makes: `proto-wire/src/wss_transport.rs:153` calls `self.gate.check(&frame, 0)` with the same `ClassicalUntilPqAudit` gate the transport constructs at `wss_transport.rs:96` / `:123`. The PoC is not a model of the path — it *is* the path's decision function.

**What genuinely improved since 2026-07-12:** the signing domain is now fixed-layout domain-separated TLV (canonical-encoding finding §4A is **closed**), and `verify_chain`/`is_fresh`/channel-binding are all *implemented correctly in isolation*. The regression is purely one of **wiring**: correct primitives, never called at the trust boundary.

---

## 2. Kill-chain re-verification table

| # | Vector | Verdict | Proof (file:line) |
|---|--------|---------|-------------------|
| 1 | **Trust anchor / self-issued caps** | **STILL OPEN** | `HybridGate::check` verifies the classical sig against `frame.capability.subject_key` (`signed_frame.rs:203` via `hybrid_gate.rs:69`) — an attacker-controlled field — and never consults a roster. `WssTransport` holds only `{ws, buf, gate}` (`wss_transport.rs:41-47`); no roster exists at the boundary. The fix (`roster.rs:225 verify_chain`) is never called (grep: only in `roster.rs` + `lib.rs:43`). PoC: NODE 1 `Ok(())`. |
| 2 | **Scope enforcement** | **STILL OPEN** | `HybridGate::check` (`hybrid_gate.rs:55-90`) reads `capability.nonce` and `pq_sig` but **never** `capability.scope`. The payload is opaque bytes; there is no "requested effect" to compare against at the gate. The subset check `requested ⊆ tail.scope` lives only in `verify_chain` (`roster.rs:283-286`), uncalled. PoC: cap scope=`Presence/Send` authorizes a `ledger.append drain=ALL` payload. |
| 3 | **Replay (cross-instance)** | **STILL OPEN** | `HybridGate.seen` is a per-instance `Mutex<HashSet<[u8;8]>>` (`hybrid_gate.rs:38`); every `connect`/`accept` builds a fresh gate (`wss_transport.rs:96,123`) → empty `seen` on each connection/node. Channel binding (F7) exists but the `recv` path (`wss_transport.rs:140-155`) **never sets or checks** it; `None` binding → zero slot "accepted by any channel" (`signed_frame.rs:158`, doc `:18-22`). PoC: NODE 2 (fresh gate) accepts identical replayed bytes. |
| 4 | **Expiry** | **PARTIAL — OPEN on the wire** | `is_fresh(now)=expiry>now` (`capability.rs:104-106`) *is* now checked in the gate (`hybrid_gate.rs:57`), but `recv` hardcodes `gate.check(&frame, 0)` (`wss_transport.rs:153`). At `now=0`, any `expiry≥1` passes. PoC: `expiry=1` accepted at `now=0`; the SAME frame is `Err(Expired)` at `now=100` — the only guard is the hardcoded `0`. The comment at `wss_transport.rs:146-152` admits expiry is "delegated to the clock-holding verifier" that does not exist on this path. |
| 5 | **PQ-leg strip / downgrade** | **PARTIAL — asymmetry remains, PQ = 0 security** | Deployed policy is `ClassicalUntilPqAudit` (`wss_transport.rs:96,123`). Gate accepts `pq_sig=None` (`hybrid_gate.rs:82-88`) and **rejects** `pq_sig=Some(_)` as `HybridIncomplete` (`hybrid_gate.rs:72-80`) — i.e. **stripping the PQ leg is still the only way in; presenting one is rejected.** `verify_pq` always errors (`signed_frame.rs:221-226`); the PQ leg is entirely unwired. "Post-quantum" security is **zero today** (honestly labeled, but the review's downgrade observation stands). |
| 6 | **Canonical encoding** | **FIXED** | Signatures now commit to fixed-layout, domain-separated, length-prefixed TLV (`tlv.rs:81-114`; `capability.rs:86-100`; `signed_frame.rs:128-146`), not `serde_json`. `serde_json` removed from `proto-cap` deps (dev-only, `proto-cap/Cargo.toml`). Per-type `DOMAIN_*` tags reject cross-structure reuse (`tlv.rs:63-70`, test `capability.rs:156-185`). The JSON-reserialization/duplicate-key/whitespace class (§4A) is genuinely closed on the signed path. (The outer `Envelope` still uses `serde_json` — but it is **not signed**, so no malleability.) |
| 7 | **Handshake / version** | **STILL OPEN** | `ENVELOPE_VERSION` (`envelope.rs:15`) is write-only: `framing::decode` (`framing.rs:38-54`) returns the `Envelope` without checking `version`; `recv` uses only `env.payload` (`wss_transport.rs:143-144`), never `env.version`. No downgrade protection. `Handshake` (`handshake.rs:36`) is **never constructed, sent, or received** (grep confirms only its own `impl`) — data flows before any authentication event. |

**Net:** 3 STILL OPEN (1,2,3), 2 PARTIAL-OPEN (4,5), 1 STILL OPEN (7), 1 FIXED (6). The three composing links needed for takeover (self-issue + scope-blind + cross-instance replay) are all fully open.

---

## 3. Findings by severity

### CRITICAL

**C1 · Self-issued capabilities accepted — root-of-trust never enforced at the gate.**
The anchored delegation model (`roster.rs`) is correct and tested but **not on the acceptance path**. `HybridGate::check` authenticates "I hold the private key for the public key I wrote in `subject_key`" — never "someone authorized that key." A fresh, unenrolled key self-authorizes any action. *Broken invariant:* an unenrolled key must not be able to mint valid authority (no root of trust at the trust boundary). *Proof:* PoC NODE 1 `Ok(())`; `hybrid_gate.rs:55-90`, `wss_transport.rs:41-47,153`.

**C2 · Scope is carried but never checked against the requested effect.**
`capability.scope` is signed and transported but the gate never reads it; the payload effect is unconstrained. A `Presence/Send` capability authorizes a ledger drain. Combined with C1 this is unconditional privilege escalation. *Broken invariant:* the authorized `(resource, action)` must bound the effect the frame requests. *Proof:* `hybrid_gate.rs:55-90` (no `scope` read); PoC payload `ledger.append drain=ALL` under scope `Presence/Send`.

**C3 · Cross-instance / cross-connection replay.**
Replay state (`seen`) is per-`HybridGate`, and a fresh gate is built per connection and per node; channel binding is implemented but neither set nor required by `recv`. The exact bytes replay to a second node (or a new connection to the same node). *Broken invariant:* a single-use nonce must be single-use across the mesh, not per-socket. *Proof:* `hybrid_gate.rs:38`, `wss_transport.rs:96,123,140-155`; PoC NODE 2 `Ok(())`.

### HIGH

**H1 · Expiry defeated on the reference carrier (`now=0`).**
The freshness check is real but the only receive path hardcodes `now=0`, so any positive expiry passes; an expired credential is accepted on the wire. *Broken invariant:* an expired capability must be rejected at ingest. *Proof:* `wss_transport.rs:153` vs `capability.rs:104-106`; PoC accepts at `now=0`, rejects (`Err(Expired)`) at `now=100`.

**H2 · Insert-before-verify unbounded `seen` growth (remote OOM).**
`check` inserts the nonce (`hybrid_gate.rs:63`) **before** verifying the signature (`hybrid_gate.rs:69`). Unauthenticated frames with unique nonces grow `seen` without bound within a connection an attacker keeps open — the review's PoC5 memory bomb, still present. Also `hybrid_gate.rs:62` `.expect("nonce set poisoned")` converts any mutex poison into a permanent per-connection panic. *Broken invariant:* remember only what you have authenticated. *Proof:* `hybrid_gate.rs:60-69`.

**H3 · "Post-quantum" is false today, and PQ-strip is rewarded.**
The PQ leg is entirely unwired (`verify_pq` always errors, `signed_frame.rs:221-226`); the deployed policy accepts a single Ed25519 signature and *rejects* any frame that carries a PQ signature. A quantum (or Ed25519-key-compromising) adversary has full forgery, and the "hybrid-only until audit" red line is not enforced at runtime. *Broken invariant:* hybrid-only until audit. *Proof:* `hybrid_gate.rs:72-89`, `wss_transport.rs:96,123`.

**H4 · Version unauthenticated and unchecked → downgrade unprotected.**
`ENVELOPE_VERSION` is never read on receive; there is no negotiation and no handshake authenticating the version. *Broken invariant:* protocol version must be checked and bound. *Proof:* `framing.rs:38-54`, `wss_transport.rs:143-144`, `envelope.rs:15`.

### MEDIUM

**M1 · Channel binding is decorative on the enforcement path.**
`channel_binding`/`sign_frame_bound`/`Handshake` are correctly implemented but appear only in tests (grep). The `recv` path never produces a transcript, never sets a binding, and never rejects `None`-binding frames — so the F7 cross-channel-replay defense does not run in production. The green test `wss_rejects_cross_channel_replay` only proves rejection when the attacker *mutates the binding field and keeps the old signature* (a signature mismatch), not that the server rejects an exact-bytes replay or a `None` binding. *Proof:* `wss_transport.rs:140-155`, `handshake.rs:36` (dead), tests at `wss_transport.rs:368-409`.

**M2 · Plaintext "WSS" server — no confidentiality (context, transport layer).**
`accept` wraps the stream as `MaybeTlsStream::Plain` (`wss_transport.rs:118`); payloads travel as cleartext JSON. Out of the strict `proto-cap`/`core` authz scope but it compounds C3/H1 (a passive tap plus cross-instance replay = trivial redirection). *Proof:* `wss_transport.rs:118`.

---

## 4. Fix (direction only — the architect fixes; this states the violated invariant)

The primitives already exist; the gap is that the **trust boundary bypasses them**. To close the takeover:

1. **Wire the root of trust into ingest (kills C1/C2).** The receive path must hold an `AnchorRoster` and run `verify_chain(roster, delegation_chain, &frame.capability, now)` — including the `requested_effect ⊆ tail.scope` check — *before* the frame is returned. That requires the frame to actually carry a delegation chain (today `SignedFrame` has no chain field), and `HybridGate`/`WssTransport` to own a roster. Until `verify_chain` is on the path, `roster.rs` is inert.
2. **Make replay state mesh-scoped and verify-before-record (kills C3/H2).** Move `seen` out of the per-connection gate into a bounded, expiry-pruned window keyed by `(subject_key, nonce)` shared across connections; insert only *after* `verify_classical` succeeds; recover from poison instead of `.expect`.
3. **Thread a real monotonic clock into `recv` (kills H1).** Delete the hardcoded `now=0`; bound `max_ttl` and skew.
4. **Enforce channel binding on receive (kills M1, hardens C3).** Reject `None`-binding frames once the handshake produces a transcript; compare the frame's binding to the live channel's transcript hash.
5. **Make hybrid structurally mandatory or stop labeling "post-quantum" (H3);** **check + bind `Envelope.version` on receive (H4).**

**Red lines honored:** none of the above introduces courier/agent scoring; the authorization surface stays per-frame identity + delegation, no reputation.

---

## Appendix — method & honesty

- PoC crate: `b2pwn` (out-of-tree), `[dependencies] bebop-proto-cap = {path=…}`, `bebop2-core = {path=…, features=["std","test_keygen"]}`. `cargo run --release` output reproduced verbatim in §1. The target tree was read-only; no file under `/root/bebop-repo/bebop2` was modified by the exploit, and no git command was run against it.
- `cargo test -p bebop-proto-cap` = **31/31 green** — the green-wash: the roster RED/GREEN tests pass in isolation while the node they were meant to protect is still captured.
- Honestly bounded: signature forgery against a key you do **not** control remains blocked (real Ed25519); the canonical-encoding defect (§4A) is genuinely fixed; every PoC frame is *validly signed under the attacker's own key* — the defect is the **trust model at the boundary**, not the primitive. This report re-verifies only the `proto-cap`/`core` authorization kill-chain; the `core` PQ-break (§3B) and numeric findings (§4C) from 2026-07-12 were out of scope here and are neither re-confirmed nor cleared.
