# bebop2 — Fable Red-Team Review (2026-07-12)

**Scope:** `/root/bebop-repo/bebop2` @ `review/proto-crypto` — crates `core`, `proto-cap`, `proto-wire`, `proto-crypto` (~8.7k LOC Rust).
**Method:** 8 parallel adversarial agents (Fable model), read-only, each mapped to an attack surface + red-team skill (owasp-security / systematic-debugging / doubt-escalation). Findings below are cross-checked across agents; the authorization break was **weaponized into running exploit code** (all PoCs compile and pass against the real crates). Confidence tags: **CONFIRMED** = traced through code / reproduced / PoC; **PLAUSIBLE** = strong inference, needs runtime.

---

## 0. Bottom line (answers to the brief)

**Is bebop2 a protocol?** **No — not yet.** It is a from-scratch post-quantum **crypto library** (`core`, genuinely substantial) plus **aspirational protocol scaffolds** (`proto-cap`, `proto-wire`, `proto-crypto`). None of the artifacts that make something a *protocol* rather than a Rust crate are present: there is no byte-level wire spec, no canonical encoding (signatures are computed over non-canonical `serde_json`), no enforced versioning/negotiation, the handshake module is dead code, there is no second implementation and no shared wire test vectors. The single normative definition of "the protocol" is the Rust type declarations themselves. Verdict: **a single-implementation would-be protocol / protocol-in-name.**

**How good is it?**
- **Classical crypto core — genuinely good.** SHA-2/3, ChaCha20/XChaCha20-Poly1305, Argon2id, Ed25519 are implemented from scratch, anchored to **real RFC/FIPS known-answer vectors**, and pass (94/94 core tests green). AEAD tag comparison is constant-time. Ed25519 is functionally correct and anti-malleable. The wasm build has an empirically-verified **empty import section**. This is real, careful work.
- **Post-quantum layer — broken and unvalidated.** The two PQ primitives that justify the entire project are not trustworthy (see §3B). This is the most important finding: the "post-quantum" claim is currently false.
- **Protocol / authorization layer — insecure, and it was weaponized.** A random attacker can self-authorize the highest-privilege action, replay it across nodes, strip the PQ leg, and bypass expiry — **proven with a running end-to-end node-takeover PoC** (§2).
- **Numeric/math core — correct only on a narrow reference envelope**, silently wrong one step outside it (§4C).

**Security posture in one line:** the break is **authorization and PQ validation, not the classical signature primitive** — the crypto foundation is sound; the trust model built on top of it is not.

### Scorecard

| Crate | Real % | Security posture |
|---|---|---|
| `core` (classical) | ~80% | **Good.** Real KATs, constant-time AEAD, zero-dep, empty-import verified. Gaps: non-CT elsewhere, no entropy source, no zeroization. |
| `core` (post-quantum) | implemented but **broken** | **Critical.** ML-DSA security destroyed; no external KAT; not FIPS/interoperable. |
| `proto-cap` (auth) | ~60% | **Critical.** Self-issued caps, no trust anchor, scope unenforced, replay, PQ leg unwired. |
| `proto-wire` (transport) | ~40% | **High.** Plaintext "WSS" server, version unenforced, DoS; iroh 100% stub. |
| `proto-crypto` (ladder) | ~5% | Placeholder structs; self-labeled "SCAFFOLD ONLY" (honest). |
| `kernel/` `cli/` `reloop/` | 0% | Vapor — referenced in README, absent from tree. |

---

## 1. Is it a protocol? — criteria

| Criterion | Verdict | Evidence |
|---|---|---|
| Wire-format specification | **PARTIAL** | Only the outer `[u32 LE len][JSON]` frame layer is documented (`proto-wire/src/framing.rs:5-8`). The body is "whatever `serde_json` emits for these structs" — no byte-layout spec exists. |
| Canonical encoding | **ABSENT** | Every signature is over non-canonical JSON (`proto-cap/src/capability.rs:53-55`, `signed_frame.rs:61-65`). Directly violates the project's own mandate (`ARCHITECTURE.md:75`: "fixed-layout… no serde"). |
| Versioning & negotiation | **PARTIAL** | `ENVELOPE_VERSION` field exists (`envelope.rs:15`) but is **write-only** — never checked on receive, unauthenticated, no negotiation, no downgrade protection. |
| Handshake / state machine | **ABSENT** | `handshake.rs:19-30` is a struct **never constructed, sent, or received** anywhere. Data flows before any authentication event. |
| Framing | **PRESENT** | The one protocol-grade layer: length-delimited, 8 MiB cap, partial-read handling, tested, panic-safe (`framing.rs:19-54`). |
| Interoperability | **ABSENT** | No spec doc (admitted missing, `proto-wah-README.md:9-12`), no second implementation, no shared wire vectors. "The protocol" == these two Rust crates. |
| Transport-independence | **PARTIAL** | `Transport` trait + shared envelope exist, but the iroh carrier is 100% stub (`iroh_transport.rs:57-79` all return `NotConnected`) — n=1 real carrier, claim untested. |

**What would make it a protocol:** a versioned byte-level wire spec independent of Rust; a canonical/fixed-layout codec for anything signed; an anchored trust model; a real handshake state machine; enforced+authenticated versioning; shared wire test vectors + a second implementation; confidentiality.

---

## 2. Attacker kill-chain — WEAPONIZED (runs today)

An exploit agent built a scratch crate that path-depends on the real crates read-only, and ran six PoCs (all pass) plus a chained takeover through the **actual** `WssTransport → serde_json → framing → gate.check` pipeline. Real `cargo run --release` output:

```
=== CHAINED SUPER-EXPLOIT: end-to-end node takeover ===
attacker generated a fresh keypair (no enrollment)
attacker put 2259 bytes on the wire (length-prefixed envelope)
NODE 1  ACCEPTED forged frame. scope=Presence/Send, payload={"op":"ledger.append","drain":"ALL","to":"attacker"}
NODE 2  ACCEPTED the SAME replayed bytes (single-use defeated)
=== takeover complete: no legitimate key, no PQ, expired, wrong scope, replayed ===
```

One forged frame — attacker's own key, PQ leg stripped, past expiry, low-priv scope masking a `ledger.append` drain — accepted by node 1 **and** replayed verbatim to node 2. The composing PoCs:

| PoC | Result | Mechanism |
|---|---|---|
| Self-issued capability forgery | **WORKS** | `verify_classical` checks the signature against `capability.subject_key` — a field the attacker fills (`signed_frame.rs:106`). No issuer, no anchor. |
| Scope non-enforcement | **WORKS** | `HybridGate::check` never reads `capability.scope` (`hybrid_gate.rs:55-90`). Low-priv cap authorizes a high-priv payload. |
| PQ-strip downgrade | **WORKS** | `pq_sig=Some(_)` → `Err(HybridIncomplete)`; `pq_sig=None` → `Ok`. Removing PQ protection is *rewarded* (`hybrid_gate.rs:72-89`). |
| Cross-instance replay | **WORKS** | `seen` nonce set is per-`HybridGate`, rebuilt per connection (`wss_transport.rs:96,123`). A fresh verifier accepts replayed bytes. |
| Unbounded nonce memory bomb | **WORKS** | Nonce inserted **before** signature verify (`hybrid_gate.rs:63` vs `:69`); 200,000 *unauthenticated* frames grow `seen` without bound → remote OOM. |
| Expiry bypass at `now=0` | **WORKS** | `recv` calls `gate.check(&frame, 0)` (`wss_transport.rs:153`); `is_fresh = expiry > now` → any `expiry≥1` passes. |

**Honestly bounded (what could NOT be weaponized):** signature forgery against a key you don't control is **blocked** (real RFC-8032 Ed25519); PQ forgery is N/A (leg unwired); same-instance replay is caught; the 8 MiB frame cap holds. Every PoC used *validly signed* frames under an attacker-chosen key — the defect is the **trust model**, not the primitive.

---

## 3. Critical findings

### 3A. Authorization is unanchored — self-issued capabilities (CONFIRMED, PoC)
`Capability.subject_key` is carried in-frame and the signature is verified against that same in-frame key, with **no allowlist / issuer / registry / delegation anywhere** in the tree. The frame proves "I hold the private key for the public key I wrote here" — not that anyone *authorized* that key. Consequences: total authorization bypass (§2), free Sybil identities (deterministic offline keygen; a fake identity costs ≈ nothing), and — for the dowiz delivery layer — the ability to self-sign `Route`/`Ledger`/`DeliveryIntent` frames to reroute deliveries, forge/desync order state (a ledger-desync primitive across partitions), or replay a "delivered" confirmation. **Root fix:** capabilities must be *delegated* — signed by an issuer key that chains to a mesh-trusted anchor — and `check()` must compare `scope` to the requested effect.

### 3B. Post-quantum crypto is broken and unvalidated (CONFIRMED)
- **ML-DSA-65 is cryptographically broken.** `expand_a` samples the public matrix **A from a centered-binomial distribution (coeffs [−4,4]) instead of uniform over Z_q** (`pq_dsa.rs:224-238`). With A, s1, s2 all small, `t = A·s1 + s2` is a near-trivial lattice instance — the Module-LWE/SIS hardness the signature rests on is destroyed. Textbook catastrophic break.
- **Neither PQ primitive has an external KAT.** ML-KEM's "dual-impl" test delegates to the production code (circular); ML-DSA's "golden KAT" is a hash of the implementation's *own* output captured from HEAD (`pq_dsa.rs:730-761`). A self-consistent wrong (or trapdoored) implementation passes its own tests. Cross-verified by 3 agents.
- **Not FIPS 203/204 / non-interoperable.** ML-DSA sizes are wrong (pk 3104 vs 1952, sig 3896 vs 3309, c̃ 32 vs 48); ML-KEM stores coefficients in the wrong domain ("NTT was found incorrect and removed"). The "ML-KEM-768 / ML-DSA-65" labels are inaccurate.
- **The "hybrid" gate is classical-only.** PQ verify is unwired (`signed_frame.rs:91-93`); the only usable policy accepts a single Ed25519 signature. A quantum adversary — or anyone who breaks the unvalidated leg — has full forgery. (Note: because the PQ leg is not wired into the protocol, the ML-DSA break is a *latent* failure that activates the moment it is turned on — but it invalidates the post-quantum claim today.)

### 3C. No entropy source — deterministic keys (CONFIRMED)
The "CSPRNG" is a deterministic ChaCha20 stream keyed by a **caller-supplied** seed (`rng.rs`); there is **no entropy gathering anywhere in-tree**, and every seed currently present is a hardcoded constant (`[42u8;32]`, `[5u8;32]`, …). If any of these shortcuts reaches production, all keys, nonces, and ephemeral KEM secrets are predictable — total break. All determinism KATs still pass, so this is invisible to the current test suite. No secret is ever zeroized (liftable from a core dump / swap).

### 3D. Transport has zero confidentiality and is replayable (CONFIRMED)
The "WSS" **server** terminates **plaintext** WebSocket (`MaybeTlsStream::Plain`, `wss_transport.rs:118`) — no TLS acceptor, no cert. Payloads travel as cleartext JSON despite ML-KEM/XChaCha existing in `core`. Combined with per-connection replay memory and disabled expiry (§2), a passive tap reads everything and an active MITM can partition/censor/redirect. The client path *does* verify TLS cert+hostname when given `wss://`, but there is no pinning and `ws://` is silently accepted (downgrade).

---

## 4. High findings

**4A. Signatures over non-canonical JSON (CONFIRMED).** Signing domain is `serde_json::to_vec(cap) ‖ payload` (`capability.rs:53-55`). serde_json is implementation-defined, not canonical — interop-fatal for anything signed, and a latent silent break the moment a map/float/`flatten`/version-skew is introduced. Also violates `ARCHITECTURE.md:75`. Not exploitable *today* only because the current struct is canonical-friendly and the verifier re-serializes the parsed struct.

**4B. Denial-of-service surface (CONFIRMED).** (i) Insert-before-verify unbounded `seen` set → remote OOM from unauthenticated input (§2, PoC5). (ii) WS message-size left at the tungstenite default (~tens of MiB) while the app cap is 8 MiB → ~8 MiB pinned per socket across unlimited connections. (iii) `hybrid_gate.rs:62` `.expect("nonce set poisoned")` turns any mutex poison into a permanent per-connection panic.

**4C. Numeric core — correct only on a narrow envelope (CONFIRMED, reproduced).** Tests verify labels, not properties:
- **Lyapunov gives the wrong stability verdict for non-symmetric systems** — symmetric-only Jacobi applied to arbitrary A; a stable damped oscillator `[[0,1],[-100,-2]]` is reported **unstable** (`lyapunov.rs:19-61`).
- **`active_diffuse` has a sign error — it anti-diffuses** (`u + dt·L·u`, `field.rs:193`); no stable dt exists, so the advertised "stable dt=0.02 corridor" is vacuous (mass explodes to 2.4e8). Inherited verbatim from old bebop → "matches the oracle" enshrined a wrong-sign bug as GREEN.
- **SpectralKalman covariance is silently wrong (~26%) for non-symmetric A** (`kalman.rs:201-227`); the claimed **Potter/Carlson square-root Kalman is not implemented** (prediction-only, no PSD test).
- Silent wrong results on **non-power-of-two** inputs (FFT/VSA/circulant-eigenvalues); FFT **panics** (OOB) on non-pow2; `active_diffuse(steps<0)` is an **infinite loop**; the `unsafe` bump allocator **aligns the offset, not the address** (UB for align>1).

**4D. Non-constant-time secret handling (CONFIRMED).** Secret-dependent `continue` in `poly_mul` (both KEM and DSA) leaks secret-key coefficient positions; Ed25519 `scalar_mul` and `mod_l` are bit-serial branchy over the secret scalar/nonce → private-key recovery under a timing/power adversary. (Note: this refines the "Ed25519 is solid" observation — it is *functionally* correct and anti-malleable, but *side-channel* leaky.)

---

## 5. Supply-chain & build-integrity

- **"Zero-dep AGC-class" is true only for `core`.** `proto-wire` pulls **66 crates** including `tokio`, `tokio-tungstenite → native-tls → openssl → openssl-sys` whose build-deps are **`cc`, `pkg-config`, `vcpkg`** — a **C compiler runs at build time** and the binary links **system OpenSSL**. `proto-cap` pulls ~10 (incl. `serde_derive`/`syn` proc-macros = arbitrary code at compile time). The protocol layer is the opposite of "empty-import, no socket/clock/rng reachable."
- **No `Cargo.lock` committed** → builds float; a compromised patch release of any of the 60 transitive crates is pulled silently on the next build.
- **RUSTSEC status is a false green:** `cargo deny check advisories` printed `advisories ok` but **no advisory database exists** on the machine. `cargo-audit`/`cargo-geiger`/`semgrep` are not installed. No CVE assertion can be trusted; `native-tls`/OpenSSL and a two-majors-behind `tokio-tungstenite@0.23` are structural risk flags.
- **The empty-import integrity gate does not exist.** README lists `reloop/` "which checks this" — the directory (and `kernel/`, `cli/`) is absent, no CI runs, and the one manual script `core/scripts/check-wasm32.sh` **`exit 0`s with a warning if `wasm-tools` is missing**, inspects the *debug* artifact, and detects imports by **string-grep, not import-section parse**. (The property itself is currently true when built — but it is unenforced and could regress or be backdoored silently.)
- **Backdoor playbook that survives every current gate:** trapdoor the ML-DSA nonce/`y` or hide `sk` bits in the non-FIPS packing slack, keep `verify` self-consistent, then **re-pin the golden KAT** from the new output — all tests stay green because no external FIPS vector contradicts it. The NO-COURIER-SCORING guard is a **grep for a banner string**; its "falsifiable" backing test is a tautological `const fn { false }`. These are **label-gates, not property-gates** — bebop's own meta-fallacy.

---

## 6. Honesty audit — claim vs reality

*"The code is honest almost everywhere the docs are not."* Every stub in the proto crates self-declares and fails closed; the README/proto-wah-README carry several claims the tree cannot back:

| Claim | Reality | Tag |
|---|---|---|
| "SKELETON ONLY — no network code" (proto-wah-README) | 320 LOC of real, tamper-tested WSS transport | STALE |
| "from-scratch, zero-dependency" (project) | true for `core`; protocol pulls 66-crate async+TLS+C stack | OVERCLAIM |
| "empty wasm import section" | **REAL** — built it, 0 imports (but the `reloop/` gate enforcing it is vapor) | REAL / gate VAPOR |
| `kernel/` `cli/` `reloop/` layout | none exist | VAPOR |
| "FIPS 203/204 KAT committed, bit-exact" | zero FIPS PQ vectors exist; PQ KATs self-generated | OVERCLAIM |
| KAT vectors "canonical ground truth" | `ED25519[0]` is cryptographically **invalid** (fabricated); an Argon2id vector pairs a real tag with fabricated inputs — both dead code no test consumes | OVERCLAIM |
| "hybrid-only until audit… classical AND ML-DSA" | classical-only in practice; PQ not even representable in `Capability` | OVERCLAIM |
| "equivalence tests vs old bebop pass" | no such package, no such tests | VAPOR |
| `rng` "CSPRNG from hardware entropy" | deterministic, caller-supplied seed, no entropy in-tree | OVERCLAIM |
| "clippy disallowed + constant-time asserts" | no `clippy.toml`; one CT compare (AEAD tag) | OVERCLAIM |
| NO-COURIER-SCORING "CI GUARD" | banner in all 20 files, no scoring surface (**red line honored in code**), but there is **no CI** | REAL posture / OVERCLAIM ("CI") |
| "cargo test GREEN" | **REAL** — core 94/94, proto-cap 9/9, proto-wire 7/7, proto-crypto 1/1 | REAL |

---

## 7. What is genuinely good (don't lose this)

- `core` is real, careful, ~80% complete: 94/94 tests, truly zero external deps, empirically empty wasm import section.
- Classical crypto anchored to **real** RFC/FIPS KATs: SHA-2/3, ChaCha20/XChaCha20-Poly1305, Argon2id, Ed25519. AEAD tag compare is constant-time; Poly1305 reduction is correct.
- Ed25519 is functionally correct and enforces canonical `S<L` (anti-malleability) and canonical `y` on decompress — signature forgery against an unknown key is blocked.
- ML-KEM (unlike ML-DSA) uses a **uniform** A — a plausibly-secure MLWE KEM, just non-FIPS and unvalidated.
- The framing decoder is panic-safe and allocation-bounded; the 8 MiB cap holds.
- The stubs are *honest* and fail-closed; the gap is in the docs and the trust model, not in hidden broken behavior.

---

## 8. Remediation ladder (ranked)

1. **Anchor identity.** Capabilities must be delegated by an issuer key chaining to a trusted anchor; reject unknown `subject_key`; add a `scope`→effect check in `check()`. (Kills §2, §3A, Sybil, delivery forgery.)
2. **Verify before you remember, and make expiry/replay real.** Reorder `check()` to verify the signature *before* inserting the nonce; bound + time-window `seen`; thread a monotonic clock into `recv` (no `now=0`); share replay state across connections keyed by `(subject_key, nonce)`. (Kills the OOM DoS + cross-instance replay.)
3. **Fix or de-label the PQ crypto.** Either make ML-KEM/ML-DSA FIPS-bit-exact and assert **official NIST ACVP vectors** (uniform A for ML-DSA, correct NTT domain and packing, 48-byte c̃), or stop calling it "post-quantum." **Ban self-captured golden bytes** as the sole KAT.
4. **Give the RNG a real, fail-closed entropy source**; forbid constant seeds; refuse to build a production profile without a wired entropy provider. Zeroize all secret material.
5. **Authenticate the channel.** Mandatory TLS (pinned) on `accept`; use `Handshake.peer_id` as a real signed, key-bound handshake; reject `ws://`. Prereq for a safe iroh peer-admission layer before iroh is wired.
6. **Canonical/fixed-layout codec for anything signed** (per `ARCHITECTURE.md:75`); enforce and authenticate the version field.
7. **Make the gates property-gates, not label-gates.** Real empty-import CI (parse the section on the *release* artifact; fail if `wasm-tools` absent), commit `Cargo.lock`, wire `cargo-audit` + a populated advisory-db + a real `deny.toml`, and replace the NO-COURIER banner-grep with a structural scan. Delete README claims for components that don't exist (`reloop/`/`kernel/`/`cli/`, FIPS KATs, equivalence tests) until they do.
8. **Numeric core:** fix the `active_diffuse` sign, use a non-symmetric eigensolver (Schur/QR) for Lyapunov+Kalman or assert symmetry, guard non-pow2 / negative-steps / small-vector edge cases, fix the bump-allocator alignment. Implement the claimed square-root Kalman or drop the claim.

---

## Appendix — method & confidence

8 Fable agents, read-only, in two waves (audit + offensive), cross-checked against each other and — for the authorization break — against **compiled, executed PoC code** (scratch crate at `/root/.claude/jobs/c6a4c73f/tmp/bebop2-pwn/`, path-dep on the real crates; the source tree was not modified and no git was run). One inter-agent conflict was resolved: Ed25519 is both functionally-correct/anti-malleable **and** side-channel-leaky — both true of different properties. Items marked PLAUSIBLE (exact fexp wrap values, some timing leaks, cross-user nonce-burn) need a runtime/timing harness to make bit-exact. Nothing here was committed or pushed to git; this document is the deliverable.
