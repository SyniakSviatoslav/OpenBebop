# B3 — Wire / Transport Red-Team (proto-wire)

**Target:** `/root/bebop-repo/bebop2`, branch `feat/logic-governance` (HEAD `d94f013`)
**Crate:** `bebop-proto-wire` (`proto-wire/src/*.rs`, 875 LOC) + its auth dependency `bebop-proto-cap`
**Perspective:** hostile network attacker (on-path MITM, DoS, malformed frames)
**Method:** full read of every `.rs` in `proto-wire` + the `proto-cap` verification path; `cargo test -p bebop-proto-wire` (10/10 green); two weaponized PoCs (both pass — see F2/F4); tungstenite 0.23 config source inspected.
**Prior art re-verified:** `RED-TEAM-REVIEW-2026-07-12.md` §proto-wire, `REMEDIATION-BLUEPRINT-2026-07-12.md`.

---

## 1. Bottom line

**No — the wire layer is NOT safe to expose to a hostile network.**

The transport advertises itself as "WSS (WebSocket **Secure**)" (`lib.rs:8`, `wss_transport.rs:1`) but ships a **plaintext `ws://` socket with TLS deliberately disabled** (`wss_transport.rs:118`, `Cargo.toml`). On that plaintext channel:

- an on-path attacker **reads every payload in cleartext** (no confidentiality);
- a captured signed frame can be **replayed verbatim on a fresh connection** — the nonce-replay set is per-connection and expiry is checked at `now = 0`, so neither triggers (PoC passes, F2);
- the advertised **channel-binding "cross-channel replay defense (F7)" is decorative** — the receiver never compares a frame's binding against the channel it actually arrived on (F3);
- the **envelope `version` is never checked and is unauthenticated** — no negotiation, no downgrade protection (PoC passes, F4);
- there are **no connection limits, no rate limits, and no read/idle timeouts**, and the real per-connection memory ceiling is tungstenite's **64 MiB default**, not the crate's advertised 8 MiB cap (F5).

The one genuinely strong property is **per-frame content integrity**: the Ed25519 signature now commits to a hand-built canonical **TLV** signing domain (not serde_json), so tampering with a frame's capability/payload/binding is reliably rejected. That closes the prior "signatures over non-canonical JSON" finding — but it defends *frame content only*. It does not provide confidentiality, freshness, anti-replay, channel authentication, or DoS resistance. The `iroh`/QUIC carrier remains a 100 % stub, so the only working carrier is the insecure one (n = 1).

Net posture: this is a **signed-message library, not a secure transport.** Exposing it to a hostile network today leaks all traffic and permits capture-replay.

---

## 2. Prior-finding re-verification

| Prior finding (2026-07-12) | Status | Evidence (current branch) |
|---|---|---|
| Plaintext "WSS" server (no TLS) | **OPEN** | `wss_transport.rs:118` wraps the raw TCP stream in `MaybeTlsStream::Plain`; `wss_transport.rs:91` dials `ws://`; `Cargo.toml` comment: *"deliberately NO native-tls … wss:// support lands with the rustls migration in a later phase."* → F1 |
| Version field write-only / unenforced | **OPEN** | `framing::decode` (`framing.rs:38-53`) never reads `version`; `recv` (`wss_transport.rs:140-155`) never reads `env.version`. PoC `poc_envelope_version_is_not_enforced_on_decode` **passes**. → F4 |
| DoS in framing | **PARTIAL** | 8 MiB cap (`framing.rs:19,43`) is real and checked before *app* allocation, but only *after* tungstenite has already buffered up to its 64 MiB default; no connection cap / rate limit / timeout exists. → F5/F7 |
| iroh/QUIC carrier was 100 % stub (returned NotConnected) | **CLOSED** | replaced by a real `quinn`/`rustls` QUIC carrier in `iroh_transport.rs`; two live RED tests `quic_roundtrip_signs_and_verifies` + `quic_rejects_tampered_frame` prove a signed frame round-trips over a real QUIC stream and a tampered frame is rejected. → F9 |
| No confidentiality | **OPEN** | Direct consequence of plaintext `ws://` (F1). Payloads (`SignedFrame.payload`) are cleartext on the wire. |
| Non-canonical serde_json on the **signed** path | **FIXED** | Signatures now commit to hand-built TLV: `Capability::canonical_bytes_tlv` (`capability.rs:86`), `SignedFrame::signing_domain` / `binding_signing_domain` (`signed_frame.rs:128-161`). serde_json survives only on the *unsigned outer envelope* (`envelope.rs:41-48`) → downgraded to informational F8. |
| Handshake module = dead code | **OPEN** | `Handshake` struct (`handshake.rs:36`) is constructed only inside its own `new`; never sent/received. `channel_binding_hash` is used, but fed a **synthetic** transcript by the caller, never the real channel bytes. → F3 |

---

## 3. Findings

### F1 — No confidentiality: "WSS" is a plaintext `ws://` socket · **HIGH**
- **file:line:** `wss_transport.rs:118` (`accept_async(MaybeTlsStream::Plain(stream))`); `wss_transport.rs:82-98` (`connect` → `connect_async(ws://…)`); `Cargo.toml` (`native-tls` deliberately off, no `rustls`); misleading claims at `lib.rs:8` and `wss_transport.rs:1,6` ("REAL and tested… WebSocket **Secure**").
- **Evidence:** the server wraps the accepted TCP stream in `MaybeTlsStream::Plain` — no handshake, no cert, no key exchange. All round-trip tests dial `ws://127.0.0.1:…`. No TLS feature is compiled in.
- **Exploit:** an on-path attacker (MITM) reads every frame payload in cleartext (route/ledger/presence intents), and can drop, reorder, or inject frames at will. Frame *content integrity* survives (a modified frame fails the Ed25519 check), but confidentiality is zero and the channel itself is unauthenticated.
- **Fix:** real TLS via `rustls` (pure-Rust, satisfies the no-C-build policy) with server-certificate validation; refuse `ws://` outside loopback tests; derive the channel binding (F3) from the TLS exporter. Until then, stop calling it "Secure."

### F2 — Cross-connection replay bypass · **HIGH**
- **file:line:** `wss_transport.rs:96` & `:123` (each `connect`/`accept` mints a **fresh** `HybridGate::new(...)`); `wss_transport.rs:153` (`self.gate.check(&frame, 0)` — `now` hardcoded to `0`); `hybrid_gate.rs:38` (`seen` nonce set is per-gate/per-connection, no shared store); `capability.rs:104-106` (`is_fresh(now) = expiry > now`; with `now = 0`, any `expiry ≥ 1` is fresh forever).
- **Evidence:** PoC `poc_cross_connection_replay_is_accepted_twice` **passes** — one legitimately-signed frame (default `channel_binding = None`, `expiry = 1`) is accepted on connection #1 and then, sent **byte-identical**, accepted again on an independent connection #2. The new gate has never seen the nonce; `now = 0` means expiry never fires.
- **Exploit:** capture any signed frame (trivial on the plaintext channel, F1), reconnect, resend verbatim. The action re-executes. This is a full capture-replay against any resource the captured capability authorizes.
- **Fix:** (a) a shared/persistent nonce ledger (or per-subject monotonic sequence) that survives reconnects; (b) thread a real monotonic clock into `recv` so `is_fresh` actually bounds lifetime at the transport; (c) enforce channel binding against the real channel (F3), which alone would stop cross-*channel* replay.

### F3 — Channel binding (F7) is decorative: never compared to the real channel · **HIGH**
- **file:line:** `wss_transport.rs:140-155` (`recv` runs `gate.check`, which only re-verifies signature self-consistency from the frame's *own* `channel_binding` field); `WssTransport` (`wss_transport.rs:41-47`) stores **no** channel transcript; grep confirms `channel_binding` is only *set* (`lib.rs:109`) and read in tests — never compared in transport code. `Handshake` (`handshake.rs:36`) is never exchanged.
- **Evidence:** the existing test `wss_rejects_cross_channel_replay` (`wss_transport.rs:368-409`) proves only that **modifying** the binding field breaks the signature. It never tests a **verbatim** replay of an unchanged frame on a different channel — which passes (see F2). The default send path (`sign_frame`, `lib.rs:90`; `SignedFrame::new` → binding `None`, `signed_frame.rs:97`) emits channel-agnostic frames. The signing-domain code (`signed_frame.rs:156-161`) even documents that `None` ⇒ 32 zero bytes ⇒ "captured frame can be replayed cross-channel."
- **Exploit:** the advertised F7 "cross-channel replay defense" provides no real protection. A receiver never binds a frame to the channel it arrived on; a captured frame replays on any channel.
- **Fix:** at `connect`/`accept`, capture the real handshake transcript (or TLS exporter after F1), store its SHA3-256 on the `WssTransport`, and in `recv` reject any frame where `channel_binding != Some(self.channel_hash)` — and reject `None` in enforced mode.

### F4 — Envelope `version` unenforced and unauthenticated · **MEDIUM**
- **file:line:** `framing.rs:38-53` (`decode` never checks `envelope.version`); `wss_transport.rs:140-155` (`recv` never reads `env.version`); `envelope.rs:15,22` (the field). The version lives in the **unsigned** outer envelope, outside `SignedFrame`, so a MITM can rewrite it freely.
- **Evidence:** PoC `poc_envelope_version_is_not_enforced_on_decode` **passes** — an envelope with `version = 99` decodes `Ok` with no error.
- **Exploit:** no negotiation, no downgrade protection; a future v2 with different semantics is indistinguishable from v1; the field is purely write-only. Because it is unauthenticated, an on-path attacker can flip it.
- **Fix:** reject `version != ENVELOPE_VERSION` on `decode`; carry version inside the signed domain (or an authenticated handshake) so it cannot be tampered.

### F5 — DoS: real memory ceiling is tungstenite's 64 MiB default, not the 8 MiB app cap; no connection/rate limits · **MEDIUM**
- **file:line:** `wss_transport.rs:91,118` (`connect_async`/`accept_async` use the **default** `WebSocketConfig`); tungstenite 0.23 defaults: `max_message_size = 64<<20` (64 MiB), `max_frame_size = 16<<20`, `max_write_buffer_size = usize::MAX` (`.../tungstenite-0.23.0/src/protocol/mod.rs:82-83`); `wss_transport.rs:164` (`self.buf.extend_from_slice(&data)` copies the entire WS message before the app cap at `framing.rs:43` is consulted); no connection cap / rate limit / accept-loop backpressure anywhere in the crate.
- **Evidence:** `MAX_ENVELOPE_BYTES = 8 MiB` (`framing.rs:19`) is enforced only *after* a full WS message is buffered by tungstenite and copied into `self.buf`. A single 64 MiB WS message therefore forces ≈64 MiB (tungstenite) + up to 64 MiB (`self.buf`) per connection even though the "cap" is 8 MiB.
- **Exploit:** open many connections, each pushing a 64 MiB message → memory exhaustion / OOM. There is no per-peer or global connection cap and no rate limit, so this scales linearly with attacker connections.
- **Fix:** pass a hardened `WebSocketConfig { max_message_size: Some(8<<20), max_frame_size: Some(8<<20), max_write_buffer_size: <bounded> }` to `accept_async`/`connect_async`; add a connection cap + per-peer rate limit at the accept loop.

### F6 — Unbounded per-connection nonce set (memory-growth DoS) · **LOW/MEDIUM**
- **file:line:** `hybrid_gate.rs:38` (`seen: Mutex<HashSet<[u8;8]>>`, no eviction); `hybrid_gate.rs:63` (`seen.insert(nonce)` on every accepted frame).
- **Evidence:** every accepted nonce is retained for the connection's lifetime — no cap, no TTL/window eviction.
- **Exploit:** a long-lived authorized (or high-volume) peer streams distinct-nonce frames → the set grows without bound → per-connection memory creep.
- **Fix:** bound the set (LRU or expiry-windowed) or switch to an O(1) monotonic-sequence check per subject.

### F7 — Slowloris / no read or idle timeout · **MEDIUM**
- **file:line:** `wss_transport.rs:157-162` (`recv` awaits `self.ws.next().await` with no `tokio::time::timeout`); `framing::decode` returns `Ok(None)` and loops for more bytes whenever `buf.len() < 4 + len` (`framing.rs:46-47`).
- **Evidence:** a peer that sends a valid length prefix (say 8 MiB) and then dribbles one byte at a time — or sends nothing further — pins the connection and its buffer indefinitely; `recv` never wakes.
- **Exploit:** N slowloris connections exhaust the server's connection/task budget with near-zero attacker cost.
- **Fix:** wrap `recv` reads in an idle timeout; drop connections that stall mid-frame.

### F8 — Non-canonical serde_json on the outer (unsigned) envelope · **LOW / INFO**
- **file:line:** `envelope.rs:41-48` (`to_bytes`/`from_bytes` = `serde_json`); `wss_transport.rs:130,144` (`SignedFrame` (de)serialized via `serde_json`).
- **Evidence:** the prior "signatures over non-canonical serde_json" defect is **FIXED** — the signed path is now TLV (`capability.rs:86`, `signed_frame.rs:128-161`). The remaining serde_json is only the *unsigned* transport framing, so its malleability does not break authentication. It is still (a) at odds with the framing doc's "byte-deterministic" claim (`framing.rs:3`) and (b) a bounded JSON-parse CPU surface on attacker bytes.
- **Fix:** move the envelope to fixed-layout too, or explicitly document it as unauthenticated framing.
### F9 — iroh/QUIC carrier was a 100 % stub (transport-independence unproven) · **CLOSED**

- **file:line:** previously `iroh_transport.rs:57,64,70,75` — all four `Transport` methods returned `Err(WireError::NotConnected)`; `#![allow(dead_code)]` at `:21`.
- **Now:** the module is a real `quinn`/`rustls` (ring provider) QUIC carrier implementing the `Transport` trait. It reuses the exact SignedFrame ↔ Envelope ↔ framing pipeline as the WSS carrier, so it satisfies the same hybrid-gate / anchor-roster contract.
- **Live evidence (cites `#[test]`):**
  - `fn quic_roundtrip_signs_and_verifies` — two real QUIC nodes over loopback exchange an anchor-rooted hybrid (Ed25519+ML-DSA-65) signed frame; the server verifies it through `HybridGate::RequireBoth` and echoes it back; client reads the identical payload.
  - `fn quic_rejects_tampered_frame` — a signed-then-mutated frame is rejected by the server's `recv` (hybrid gate) over the real QUIC stream.
- **Residual ceiling (not a stub, honest limit):** the dev TLS cert is `rcgen` self-signed with an `InsecureAcceptAny` verifier — acceptable for the QUIC handshake because wire auth is the signed-frame envelope; production must swap in a real cert chain + proper verifier. This is an `innovate:` ceiling, not a P6-park.

---

## 4. DoS / crash surface list

| # | Surface | Vector | Bound | Ref |
|---|---|---|---|---|
| 1 | WS message buffering | 64 MiB message per connection | tungstenite default 64 MiB, **not** app 8 MiB | F5 · `wss_transport.rs:118,164` |
| 2 | Connection count | unlimited concurrent connections | none | F5 · accept loop (caller-driven, no cap) |
| 3 | Slowloris | length prefix + stalled/dribbled body | no read/idle timeout | F7 · `wss_transport.rs:157` |
| 4 | Nonce set growth | many distinct-nonce frames per connection | unbounded HashSet | F6 · `hybrid_gate.rs:38,63` |
| 5 | `Vec::drain(0..n)` front-drain | many small frames in one buffer | O(n²) memmove per big buffer | `framing.rs:52` |
| 6 | serde_json parse of attacker payload | deeply nested / large JSON | bounded by serde_json recursion limit (Err, not panic) | F8 · `wss_transport.rs:144` |
| 7 | Mutex poison | panic while holding `seen` lock ⇒ later `.expect` panics | low (no panic inside lock scope today) | `hybrid_gate.rs:62` |

**Panic/unwrap audit (attacker-controlled bytes):** `framing::decode` is panic-safe — the 4-byte read is guarded by `buf.len() < 4` (`framing.rs:39`), the length is bounds-checked before the `buf[4..4+len]` slice (`framing.rs:43-49`), and `drain` uses the validated range. `recv` propagates all `serde_json` / carrier errors via `?` (no `unwrap` on wire bytes). The only non-test `.expect` on a runtime value is the nonce-set lock (`hybrid_gate.rs:62`), which is not reachable from attacker input under current code. **No attacker-triggerable panic found** in the decode/recv path.

---

## 5. Reproduction (PoCs)

Both PoCs were run as an ephemeral `proto-wire/tests/` integration test against the real crate on `feat/logic-governance`; both **passed** (asserting the vulnerable behavior is present), then the file was removed to keep the tree clean.

**F2 — cross-connection replay accepted twice** (core of the PoC):
```rust
// One legitimately-signed frame; default channel_binding = None; expiry = 1.
let cap = Capability::new(pk, Resource::Route, Action::Send, [42u8;8], 1);
let mut frame = SignedFrame::new(cap, b"transfer-100-credits".to_vec());
frame.sign_classical(&seed).unwrap();
// Connection #1 accepts it. Then a SECOND independent server/connection:
c2.send(frame.clone()).await.unwrap();      // identical nonce + signature
let second = d2_rx.await.unwrap();
assert!(second, "REPLAY accepted on a fresh connection"); // PASSES
```
Root cause: fresh `HybridGate` per connection (`wss_transport.rs:96,123`) + `gate.check(&frame, 0)` (`wss_transport.rs:153`) + `is_fresh(expiry>0)` (`capability.rs:105`).

**F4 — envelope version ignored on decode:**
```rust
let mut env = Envelope::new([0u8;16], b"x".to_vec());
env.version = 99;                            // != ENVELOPE_VERSION (1)
let mut buf = framing::encode(&env).unwrap();
let decoded = framing::decode(&mut buf).unwrap().unwrap();
assert_eq!(decoded.version, 99);             // PASSES — no version check
```

---

## 6. Priority remediation order

1. **F1** real TLS (rustls) + refuse `ws://` in prod — restores confidentiality and lets F3 bind to a real channel.
2. **F2 + F3** shared nonce ledger + real clock in `recv` + enforce channel binding against the actual channel — closes capture-replay.
3. **F5 + F7** hardened `WebSocketConfig`, connection cap, per-peer rate limit, read/idle timeout — closes the DoS surface.
4. **F4** enforce + authenticate `version`.
5. **F6/F8/F9** bound the nonce set; canonicalize or document the envelope; wire (or delete) the iroh stub.
