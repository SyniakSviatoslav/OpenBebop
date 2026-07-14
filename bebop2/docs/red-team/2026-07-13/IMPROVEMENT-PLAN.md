# bebop2 · Consolidated Improvement Plan (2026-07-13 red-team → fix)

> Source reports: `docs/red-team/2026-07-13/B{1..4}-*.md` + `MASTER-SYNTHESIS.md`.
> Method note: every report claim was re-verified against the LIVE tree at
> `feat/logic-governance` @ `d94f013` BEFORE being scheduled. Per AGENTS.md
> ("ground truth outranks plans"), the plan separates **STALE (already fixed
> on this branch)** from **OPEN (still true / not yet wired)**.

## 0. Ground-truth overlay (what the reports got WRONG vs the live tree)

These CRITICALs from B1/B2/B3/B4 are **already remediated on `d94f013`** and
verified by `cargo test -p bebop-proto-wire` (11/11 green, incl.
`wss_rejects_self_signed_frame_over_real_carrier`):

- **Node takeover (B2 C1 / B3 / B4 F1):** `wss_transport.rs:172-173` now calls
  `gate.check(&frame, &self.roster, &frame.delegation_chain, now)`; `hybrid_gate.rs:91`
  calls `verify_chain(...)`. Self-signed frames are rejected `UnknownIssuer` on the
  REAL carrier. ✅ CLOSED.
- **Expiry `now=0` (B2 H1 / B3 F2):** `recv` uses real wall-clock `now`
  (`wss_transport.rs:168`). ✅ CLOSED.
- **PQ leg inverted / stubbed (B1 F3 / B3 H3):** `hybrid_gate.rs:96-108` now runs a
  REAL `verify_pq` (ML-DSA-65). `RequireBoth` is enforced; `ClassicalUntilPqAudit`
  accepts classical-only but never fakes PQ. The *primitive* is real; the *protocol
  default* is still transitional (correct per "hybrid-only until audit"). ✅ CLOSED
  on the in-force defect; the wire policy default is a deliberate, labeled ramp.
- **serde_json on signed path (B3 prior):** signatures commit to hand-built TLV
  (`signed_frame.rs`, `capability.rs`). ✅ CLOSED (outer envelope still serde_json,
  but unsigned — informational only).

The reports are dated the SAME day as the fix branch; they audited a transient state.
The honest lesson (shared with the operator): **status tables + live `cargo test`
must be the only source of truth; dated narrative reviews rot within hours.**

## 1. Sequencing (operator-mandated)

1. **FINISH the decentralized mesh post-quantum local protocol first** (bebop2) — this plan's §2.
2. **THEN fix dowiz** — this plan's §3 (dowiz repo copy).
3. **THEN close the gaps between them** — §4.

Core direction (MANIFESTO C1–C13, DECISIONS D0/D1/D6) is **preserved**. The D6
business "kill the protocol" recommendation is **explicitly NOT actioned** — it
contradicts the ratified MANIFESTO/DECISIONS precedence (newest plan outranks older;
the 2026-07-12 MANIFESTO supersedes the 2026-07-11 roadmap). Acknowledged as a risk,
documented, declined.

## 2. bebop2 — OPEN findings to fix (mesh protocol hardening)

Priority P0 = wire-safety / hostile-network; P1 = crypto correctness; P2 = polish.

### P0 — expose only to a network you trust today
- **F1/F8 plaintext "WSS":** `wss_transport.rs:133` wraps the server stream in
  `MaybeTlsStream::Plain`. MITM reads all payloads. **Fix:** add a `rustls`
  `TlsAcceptor` path; refuse `ws://` outside loopback in prod; derive channel binding
  from the TLS exporter. Until then, stop calling it "Secure" in docs/`lib.rs`.
- **F3 channel binding decorative on accept:** `connect`/`accept` never run a real
  `Handshake`; the binding field is only exercised by tests with a *synthetic*
  transcript. **Fix:** perform the handshake on `connect`/`accept`, store
  `SHA3-256(transcript)` on `WssTransport`, and in `recv` reject `None` (enforced
  mode) + any `Some(b)` where `b != self.channel_hash`. Keep optional mode for tests.
- **F4 version unenforced:** `framing::decode` never checks `version`. **Fix:** reject
  `version != ENVELOPE_VERSION` on decode; move version into the signed domain so it
  cannot be MITM-flipped.
- **F5/F7 DoS:** tungstenite default 64 MiB message cap (not the crate's 8 MiB app
  cap); no idle timeout (slowloris). **Fix:** pass hardened `WebSocketConfig
  { max_message_size: Some(8<<20), max_frame_size: Some(8<<20), max_write_buffer_size }`
  to `accept_async`/`connect_async`; wrap `recv` reads in a per-frame idle timeout;
  add an optional connection cap at the accept loop.
- **H2 (B2) insert-before-verify + OOM + panic:** `hybrid_gate.rs:82-85` inserts the
  nonce BEFORE `verify_chain`/`verify_classical`; `.expect("nonce set poisoned")`
  converts a mutex poison into a permanent per-connection panic; the `seen` set is
  unbounded. **Fix:** verify-then-record; bound the set (LRU / expiry-windowed or
  per-subject monotonic sequence); replace `.expect` with a recoverable error.

### P1 — PQ crypto correctness (do NOT fake-green)
- **F1/F2 ML-KEM-768 not FIPS-203-interoperable + no external KAT:** stores `t`/`s` in
  coefficient domain (`pq_kem.rs:473-474,604,616,622`); only ML-DSA ACVP vectors
  vendored (`core/kat/acvp/`), no `encapDecap`/`keyGen` ML-KEM vectors. **Fix (real,
  not faked):** (a) vendor NIST ACVP ML-KEM-768 vectors and assert byte-exact
  `ek/dk/ct/K` mirroring `acvp_tests.rs`; (b) re-derive a verified constant-time NTT
  with `intt∘ntt==id` AND `mul_ntts==schoolbook` gates, store `t̂`/`ŝ` in NTT domain.
  Until (a) lands, relabel "FIPS 203" → "FIPS 203-interop PENDING" in all docs/code.
- **F4/F5 ML-KEM timing side-channels:** secret-dependent `continue` + var-time `%`
  in `poly_mul`; non-CT `==` compare in `decaps`. **Fix:** constant-time NTT + Barrett
  reduction + `subtle`-style CT compare/select (resolved together with F1).
- **F6 no zeroization:** wrap secret buffers in a zero-dep `Zeroizing<[u8;N]>`.
- **F3 (B1) wire ML-DSA into the protocol verification path:** already callable; flip
  the default production policy to `RequireBoth` once identities carry ML-DSA keys
  (gated behind the KEM fix so the wire is genuinely hybrid).

### P2 — integrity / honesty
- **F9 build:** add a workspace `Cargo.toml` + committed `Cargo.lock` + `--locked` CI;
  delete README claims for non-existent `kernel/`/`cli/`/`reloop/` (or build the
  minimal `reloop/` empty-import gate).
- **F9 (B1) "Anu QRNG" vapor:** delete the claim from commit/README; keep `EntropyRng`.
- **F8 outer envelope serde_json:** document it as unauthenticated framing, or move to
  fixed-layout.

## 3. dowiz — OPEN findings (see dowiz repo `docs/red-team/2026-07-13/IMPROVEMENT-PLAN.md`)

## 4. Cross-cutting gaps
- Reporting integrity: every status claim must be backed by a `cargo test` / `pnpm test`
  that exercises the property on the LIVE path (not an isolated unit). Add a CI lint
  that greps status tables for "CLOSED" and fails if the named test does not exist /
  does not touch the transport path.
- Memory-first: record each fix + its verification hash in the corpus before merging.

## 6. Self-retro (Feynman §16 / ponytail:LESSON)

ponytail:/LESSON — A dated red-team narrative (even same-day) rots faster than the
code it audits: this 2026-07-13 sweep flagged node-takeover + PQ-inversion +
`now=0` expiry as OPEN CRITICALs, but all three were already fixed on `d94f013`
and proven by `cargo test` over the live carrier. Lesson: **status claims must be
derived from a live test that exercises the property on the real path, never from
a written review.** The one-line reporting-integrity fix (§4) — a CI lint that fails
any "CLOSED" claim without a matching live-path test — is the durable correction,
not another round of narrative review.

## 5. Out of scope (explicitly declined per manifesto precedence)
- D6 "abandon the protocol / pivot to GloriaFood clone": declined. MANIFESTO C1–C13 +
  DECISIONS D0/D1/D6 (2026-07-12) outrank the 2026-07-11 roadmap and the dated
  business teardown. The business-risk points are logged as risks, not actioned as
  direction changes.
- The `attic/` revenue stack stays quarantined until §2 (protocol) is green; revival
  gates (RLS reactivation) are tracked but not executed this pass.
