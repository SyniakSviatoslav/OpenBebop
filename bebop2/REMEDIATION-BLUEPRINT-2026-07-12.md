# bebop2 — Remediation & PQ Best-Practices Blueprint → 100%

**Companion to:** `RED-TEAM-REVIEW-2026-07-12.md` (the 8-agent adversarial review this closes).
**Method:** 4 Opus research agents mapped confirmed findings → implementation-ready fixes, each with a **standard reference**, a **RED+GREEN verification test**, and an explicit **"= 100%" acceptance** criterion. Every "GREEN" gate is an **external property** (NIST ACVP / RFC / Wycheproof / dudect / a second implementation) — never self-captured golden bytes.

**Maturity today → target:** core ~80% · proto-cap ~60% · proto-wire ~40% · proto-crypto ~5% · kernel/cli/reloop 0% **→ 100% each.**

Two hard red lines are preserved throughout: **(1) no courier/agent scoring** — now enforced by a structural AST scan, not a banner grep; **(2) hybrid-only-until-audit** — now made *structurally mandatory* (both signature legs non-`Option`), not a silent classical-only default.

---

## 1. The dependency spine (what unblocks what)

Several fixes are shared prerequisites. Land the **foundations** first or downstream work sits on sand:

```
F1  Entropy source (core/rng.rs)  ─────────────┐  blocks ALL keygen, KEM ephemerals, nonces, the KEM session
F2  ML-DSA-65 FIPS fix + NIST ACVP anchoring ──┤  blocks every pq_sig (proto-cap PQ leg, composite hybrid) & the "post-quantum" label
F3  Anchor roster (root of trust) ─────────────┤  shared by proto-cap delegation + proto-wire peer admission + mesh Sybil gate
F4  Canonical TLV signing input ───────────────┤  blocks proto-cap signatures, handshake auth, version binding
F5  Workspace + Cargo.lock + property-gate CI ─┤  blocks trustable verification of everything else (no false-green)
F6  rustls migration (drop native-tls/OpenSSL)─┤  shared by transport confidentiality + supply-chain (bans openssl-sys)
F7  Channel binding (handshake transcript h) ──┘  threads handshake → replay defense → capability signing
```

**Rule of order:** nothing is called "post-quantum" until **F2** passes ACVP; no `pq_sig` is trusted until **F1+F2**; no transport is called "secure" until proto-cap's **F3+F4+F7** land (a confidential channel to an *unauthenticated, self-minted* peer still lets it self-authorize).

---

## 2. Sequenced roadmap

| Phase | Deliverable | Gates |
|---|---|---|
| **0 — Foundations** | Workspace+`Cargo.lock`; property-gate CI (`cargo deny` fetch-then-check, `cargo-audit`); real empty-import gate; **entropy source**; NIST ACVP KAT harness; `AnchorRoster` type; TLV signing codec | F1, F4, F5 |
| **1 — Crypto correctness** | ML-DSA fix (uniform A, sizes, c̃, hints, NTT); ML-KEM NTT-domain; constant-time everywhere; zeroization; X-Wing + composite signature | F2 |
| **2 — Protocol trust** | Delegated capabilities + roster anchor; verify-then-record + channel-binding replay; PQ-Noise handshake + authenticated versioning | F3, F7 |
| **3 — Transport** | rustls TLS 1.3 (X25519MLKEM768); peer admission; DoS hardening | F6 |
| **4 — Mesh** | Sybil/eclipse admission gate **before** wiring iroh | (F3) |
| **5 — Numeric + honesty** | Numeric correctness fixes; minimal real `reloop/` or honest deletion of `kernel/`/`cli/` claims; doc-truth scan | — |

---

## 3. Remediation by component

### 3A · core — Post-Quantum crypto → 100%
*Standards: FIPS 203 (ML-KEM), FIPS 204 (ML-DSA), finalized 2024-08-13; NIST ACVP vectors `usnistgov/ACVP-Server/gen-val/json-files`.*

**ML-DSA-65 (`pq_dsa.rs`) — the catastrophic break + conformance:**
- **A uniform, NTT-domain** — replace `sample_poly_cbd` in `expand_a` (:224) with **RejNTTPoly** (FIPS 204 Alg 30): SHAKE128 over `ρ‖s‖r`, take 3 bytes → 23-bit `z`, accept iff `z<q(8380417)`; result is already NTT-domain (do not transform).
- **Secrets uniform** — `expand_s` (:200): **RejBoundedPoly** (Alg 31), 2-byte LE nonce, nibble accept iff `<9`, coeff `=4−b`.
- **c̃ = 48 B** and **SampleInBall from a SHAKE256 XOF** (Alg 29) — no wrapped-buffer bias; τ=49 nonzero ±1 coeffs.
- **verify checks the hint** — `HintBitUnpack` (Alg 21) rejects Σh>ω(55), non-increasing indices, nonzero slack.
- **FIPS packing + import API** — pk **1952** (`ρ‖SimpleBitPack(t1,10-bit)`), sk **4032** (tr = **64** bytes), sig **3309** (`c̃48‖BitPack(z,20-bit)‖HintBitPack 61B`); `μ=H(tr‖M',64)`, `ρ''=H(K‖rnd‖μ,64)`; add `{pk,sk,sig}_from_bytes` so external vectors can be imported (their absence is *why* the code fell back to self-KATs).
- **NTT + branchless** — ML-DSA NTT (ζ=1753), Barrett/Montgomery reductions; delete the secret-dependent `if x==0 {continue}` in `poly_mul_schoolbook` (:152).
- **Verify:** ACVP keyGen/sigGen byte-exact, sigVer accept+reject (incl. malformed-hint negatives), χ² uniformity on A, dudect |t|<4.5 on basemul. **=100%:** every ACVP vector byte-exact + import round-trip identity.

**ML-KEM-768 (`pq_kem.rs`)** *(keep the already-correct uniform A):*
- **NTT-domain t̂/ŝ** with the correct incomplete Kyber NTT (ζ=17) + BaseCaseMultiply (Alg 12); byte-encode NTT-domain values (:588/:594 currently store coefficient-domain → non-interoperable).
- **Constant-time FO decaps** (:665): branchless `ct_eq` accumulate + `cmov` select of `K` vs implicit-reject `J(z‖c)`.
- **Verify:** ACVP keyGen/encapDecap byte-exact incl. implicit-rejection; dudect on the compare. **=100%:** ACVP byte-exact + |t|<4.5.

**Ed25519 (`sign.rs`) — keep RFC-8032 correctness & anti-malleability (`S<L`, canonical y); make it constant-time:** fixed-window/Montgomery-ladder `scalar_mul` with `cmov` (no bit branch), **Barrett** `mod_l` (no data-dependent iteration). **Verify:** RFC 8032 §7.1 still byte-exact + dudect |t|<4.5 on `sign`.

**Kill the circular KATs (D):** vendor official **NIST ACVP** (+ pq-crystals `.rsp`, project-wycheproof) under `core/src/kat/acvp/`; strike the self-hash golden (`pq_dsa.rs:730-761`) and circular dual-impl (`pq_kem.rs:846`). **Mutation-test CI:** flip a zeta / revert A to CBD / remove a cmov → each must turn ≥1 external vector RED, else the gate isn't property-complete.

**Hybrid (G):**
- **KEM combiner = X-Wing** (ML-KEM-768⊕X25519), *not* a hand-rolled XOR/concat: `ss = SHA3-256(ss_M ‖ ss_X ‖ ct_X ‖ pk_X ‖ 0x5c2e2f2f5e5c)`; sizes ek **1216** / ct **1120** / ss **32**. Follow the draft exactly (its proof depends on ML-KEM's FO transform). Verify against X-Wing draft vectors.
- **Composite signature = ML-DSA-65 ⊕ Ed25519** (`id-MLDSA65-Ed25519-SHA512`): `M' = Domain‖len(ctx)‖ctx‖SHA-512(M)`, sig = `MLDSA.Sign(M')‖Ed.Sign(M')`, **verify = AND (both must verify)**. This makes the PQ-strip downgrade *unconstructable*. Verify: a valid-Ed/absent-MLDSA sig is **rejected**.
- **Structurally mandatory hybrid** — both legs are non-`Option` fields, so a single-leg credential can't be built; downgrade only behind an off-by-default `audited` flag.

### 3B · core — Entropy, RNG, zeroization, AEAD nonce → 100%
- **One fail-closed entropy entry** (`rng.rs`): platform `getrandom(2)` / `RDRAND` (blocking-until-init) / wasm `crypto.getRandomValues` (the *one* sanctioned wasm import); seed a ChaCha20 DRBG from it; reseed on volume/fork; **relabel `from_seed` test-only** (`#[cfg(test)]`/`dangerous_deterministic`). A release profile without a wired provider **fails to compile**; production keygen returns `Err` if entropy is unavailable — never a constant fallback. *Ref: SP 800-90A/B/C.*
- **Zeroize all secrets** — volatile write + `compiler_fence(SeqCst)` on drop for ML-DSA sk, ML-KEM dk/shared-secret, Ed25519 scalar/seed, Argon2 matrix, DRBG state (dep-free, `no_std`). Verify: post-drop zero-scan + release-codegen check.
- **AEAD nonce** (`aead.rs`) — a `SealingContext` that owns the key and **generates the 24-byte nonce internally** from the entropy source per message (XChaCha's 192-bit random nonce is the design point); raw `(key,nonce)` API `#[doc(hidden)]`/test-only. Verify: same plaintext → different ciphertext; no session nonce reuse.

### 3C · core — Numeric correctness → 100%
| Item | Fix | Verify (RED today → GREEN) |
|---|---|---|
| **Lyapunov** (`lyapunov.rs:19`) wrong verdict, non-symmetric | Hessenberg + **Francis double-shift QR** (real Schur); margin = max Re(λ); keep Jacobi fast-path for symmetric only | `[[0,1],[-100,-2]]` → **stable**, margin ≈ −1 |
| **Kalman** (`kalman.rs`) ~26% wrong; sqrt-filter only a label | Fix eigensolver (share B1) + implement **Potter/Carlson square-root** filter (`P=SSᵀ`) with measurement update | P stays PSD across a stress trajectory; non-sym A matches dense oracle |
| **active_diffuse** (`field.rs:193`) anti-diffusion | `u − dt·L·u` (sign); **CFL cap** `dt_max=2/(coeff·λmax)`; remove ±1e6 mask; guard `steps<0` | energy monotonically **decays**, mass conserved; corridor tested at its boundary; `steps=-1` returns, no hang |
| **non-pow2** (`fft.rs:123/216`, `vsa.rs`) panic / silent wrong | `fft` asserts pow2 (no OOB); **Bluestein** for arbitrary n; `circulant_eigenvalues` = size-n DFT; bind/unbind = size-n or require pow2 | non-pow2 matches `dft_oracle` to 1e-12 or errors cleanly |
| **bump allocator** (`lib.rs:44`) aligns offset not address (UB) | Over-align heap (`#[repr(align(64))]`) or align the real address; `fetch_update` for `NEXT`; `addr_of_mut!` | `ptr % align == 0` for align∈{1..64}; Miri-clean |
| **cosine_similarity** (`algebra.rs:43`) overflow/underflow | `fsqrt(na)*fsqrt(nb)` (never `fsqrt(na*nb)`); per-vector zero guard; clamp[−1,1] | `cos(a,a)=1` at 1e±200; scale-invariant |
| **fexp** (`lib.rs:331`) i64→i32 wrap for \|x\|≳1.49e9 | clamp `k` in i64 to [−1100,1100] before cast; delete duplicate `fexp_local`, delegate to `crate::fexp` | saturates monotonically to INF/0 at IEEE thresholds |

### 3D · proto-cap — Authorization → 100%
- **Trust anchor = UCAN-subset delegation chain rooted in an enrolled `AnchorRoster`** (over raw hybrid keys; fixed-layout, not DAG-CBOR). `verify()` enforces: root issuer ∈ roster (**kills self-issue**) → chain alignment `child.iss==parent.aud` → narrow-only `cmd`/window attenuation → tail `aud==subject_key` → **`requested_effect ⊆ tail.cmd`** (makes dead `ScopeViolation` live). Honest bound: authorization *needs* a root of trust — "no central issuer" = **no central issuer at runtime, one enrolled anchor at genesis**. *Ref: UCAN 1.0 Delegation; SPKI/SDSI RFC 2693.*
- **Canonical signing (F4)** — fixed-layout, domain-separated, length-prefixed **TLV** signing input (honors `ARCHITECTURE.md:75` "no serde"): `DOMAIN_TAG(16)‖struct_tag‖wire_version‖field_count‖[field_id‖u32_le len‖bytes]…`, payload signed as `sha3_256(payload)`, channel_binding as a signed field. Per-type domain tags kill cross-structure signature reuse. dCBOR/CDE (RFC 8949 §4.2) as the documented interop encoding; **JCS rejected** (ES6 float footguns).
- **Replay/freshness (F7)** — **verify-then-record** (kills unauthenticated OOM); **channel-bind the capability** so a frame signed for session A fails at fresh verifier B *cryptographically, zero shared state* (the only mesh-viable defense); bounded expiry-pruned `ReplayWindow` keyed by `(subject,nonce)`, fail-closed when full; **real monotonic clock** into `recv` (delete `now=0`), `max_ttl`+`skew` bounds, optional per-issuer monotonic counters. States honestly: full cross-instance nonce dedup needs a shared store; don't claim it leaderless.
- **Handshake + versioning** — **KEM-based PQ Noise** (KK steady / IK join) built from in-tree ML-KEM-768 + Ed25519 (no OpenSSL): auth-before-data (revives dead `handshake.rs`), version list in the Noise **prologue** bound into transcript `h` → a rewrite fails auth (downgrade protection, generalizes TLS 1.3's sentinel); `envelope.version` checked on receive; `ws://` rejected. Export `h` as the channel binding consumed above.

### 3E · proto-wire — Transport & mesh → 100%
- **Confidential authenticated channel** — **mandatory rustls TLS 1.3 on `accept()`** (delete the `MaybeTlsStream::Plain` path, `:118`); switch `tokio-tungstenite` off `native-tls` → **rustls** (cuts the `cc`/`pkg-config`/system-OpenSSL supply chain **and** gives standards-track PQ-hybrid **X25519MLKEM768**, codepoint `0x11EC`, via `rustls-post-quantum`). Client: reject non-`wss://`, pin via custom `ServerCertVerifier` / **RFC 7250 raw public keys** or mTLS. *Optional 2nd layer:* KEM-derived XChaCha20 payload session (X-Wing) — **gated** on FIPS-valid ML-KEM + an X25519 primitive (absent in-tree) + entropy.
- **Peer identity** — signed, **TLS-exporter channel-bound** handshake (RFC 5705/9266) as first exchange; `node_id = sha3_256(anchored Ed25519 pubkey)`; roster-checked; wire identity == capability `subject_key`.
- **Sybil/eclipse (before iroh)** — **enrolled/vouched roster** (Sybil cost ~0 → one revocable anchor signature) + **S/Kademlia** crypto-puzzle node-ids and **α≥3 disjoint lookup paths** (99% lookup success at 20% adversarial); layer on iroh's dial-the-key QUIC auth. Admit to routing/gossip **only after** admission check.
- **DoS** — clamp `WebSocketConfig` `max_message_size`/`max_frame_size` to the 8 MiB frame cap; bound the reassembly buffer; per-IP conn + **GCRA rate limits** (`tower_governor`) + `Semaphore` backpressure; replace `hybrid_gate.rs:62` `.expect` with `into_inner()` poison recovery (or `parking_lot`).
- **Safe iroh wiring gate** — do **not** flip `NotConnected` → live until: admission gate exists, node key anchored, proto-cap trust anchor landed, α-disjoint lookups green at f≥20%, DoS controls ported to QUIC streams, signed handshake over the QUIC exporter, and `Cargo.lock`+audit in place.

### 3F · proto-crypto — Verification ladder → 100%
Turn the 5% placeholder into a **3-rung property-gate** a trapdoor can't pass:
1. **`fips_kat`** — vendored **NIST ACVP** FIPS 203/204, byte-exact (a CBD-A or coefficient-domain-KEM trapdoor fails NIST's own outputs it never saw).
2. **`wycheproof`** — project-wycheproof accept/reject incl. malformed edge cases (wrong-len c̃, over-weight hint, non-canonical S, invalid ciphertext).
3. **`constant_time`** — real **dudect** Welch t-test, **fail if |t|>4.5** on every secret-dependent op.
Plus: **mutation-test CI** (each seeded fault turns ≥1 rung RED); replace the tautological `has_scoring_field()->bool{false}` with a **structural AST scan** (NO-COURIER by shape, not banner). External-source-only rule for every vector.

### 3G · build-integrity / CI / supply-chain / reloop-kernel-cli → 100%
- **Workspace + lock** — add root `[workspace]` (the four crates are standalone today, so there's no lock); commit **`Cargo.lock`**; all CI/release `--locked`; fix `README.md:61,64` (`-p bebop2` doesn't exist). *Ref: Rust supply-chain guide.*
- **Real empty-import gate** — a `reloop/` Rust binary parsing the **release** wasm import section with **`wasmparser`** (parse, not grep), **fail-closed** (delete the "skip if `wasm-tools` missing → exit 0" hole), with a committed RED fixture (a crate that imports a host fn) proving *rejection*.
- **Reproducibility** — `--remap-path-prefix` + `SOURCE_DATE_EPOCH` + `cargo auditable`; two-path rebuild → identical wasm hash.
- **Supply chain** — commit **`deny.toml`** (advisories+bans+sources+licenses); CI **`cargo deny fetch` then check** (DB present → no false green); add `cargo audit`; **ban `openssl-sys`/`native-tls`** (satisfied by the rustls migration F6); one RED fixture per check.
- **KAT provenance** — every KAT carries an external `source` citation **and** a consuming test (no dead vectors); ban self-captured PQ golden bytes; **fix the two fabricated vectors** — `ED25519[0]` (real RFC 8032 §7.1 sig) and `ARGON2ID` (RFC 9106 inputs `01×32/02×16/03×8/04×12`) — or delete; `provenance_scan` required.
- **reloop/kernel/cli** — **build minimal `reloop/`** (empty-import gate + bit-exact-KAT wasm executor + `.text` bound); **`kernel/`**: build with a replay test **or delete the claim**; **`cli/`**: delete the claim unless built; a **`doc-truth` scan** asserts every path named in README's layout exists.

---

## 4. Cross-cutting standards summary

| Concern | Adopt | Reject / why |
|---|---|---|
| PQ KEM hybrid | **X-Wing** (ML-KEM-768⊕X25519), draft-connolly-cfrg-xwing | hand-rolled XOR/concat (no robustness proof) |
| PQ signature hybrid | **Composite ML-DSA-65⊕Ed25519**, both-must-verify, domain-sep (draft-ietf-lamps-pq-composite-sigs) | OR-gate / PQ-optional (the weaponized downgrade) |
| TLS | **rustls 1.3 + X25519MLKEM768** (draft-ietf-tls-ecdhe-mlkem) | native-tls/OpenSSL (C build surface) |
| Capabilities | **UCAN-subset delegation + enrolled roster** | self-issued keys (the break); macaroons (shared secret); VC/JSON-LD (serde) |
| Canonical signing | **fixed-layout TLV** (+ dCBOR interop) | serde_json (non-canonical); JCS (float footguns) |
| Handshake | **KEM-based PQ Noise** (KK/IK) | plaintext / dead handshake |
| Sybil/eclipse | **enrolled roster + S/Kademlia** (puzzles, α disjoint paths) | free self-minted node ids |
| KAT anchor | **NIST ACVP + Wycheproof + dudect + mutation-CI** | self-captured golden / circular dual-impl |
| Gates | **property-gates** (wasmparser parse, AST scan, external vectors) | label-gates (string grep, `const fn{false}`) |

---

## 5. Consolidated "→ 100%" checklists

**core — PQ crypto:** ☐ A uniform NTT-domain ☐ secrets uniform ☐ c̃48 + XOF SampleInBall ☐ hint validity ☐ FIPS packing 1952/4032/3309 + import API ☐ ML-DSA NTT branchless ☐ ML-KEM NTT-domain + BaseCaseMultiply ☐ CT FO decaps cmov ☐ Ed25519 CT scalar_mul/Barrett ☐ ACVP vectors replace self-KATs ☐ mutation-CI kills every fault ☐ X-Wing ☐ composite sig both-verify ☐ hybrid structurally mandatory

**core — entropy/numeric:** ☐ fail-closed entropy entry ☐ deterministic seed test-only ☐ zeroize all secrets ☐ AEAD internal nonce ☐ Lyapunov Schur solver ☐ square-root Kalman + PSD test ☐ diffusion sign + CFL + steps guard ☐ Bluestein/pow2 guards ☐ allocator address-align + atomic ☐ cosine split-root ☐ fexp clamp

**proto-cap:** ☐ AnchorRoster + delegation chain ☐ scope⊆effect enforced ☐ TLV canonical signing (no serde) ☐ verify-then-record ☐ channel-bound capability ☐ bounded pruned replay window ☐ real clock + max_ttl + skew ☐ PQ-Noise KK/IK handshake ☐ auth-before-data ☐ version bound into transcript

**proto-wire:** ☐ rustls TLS on accept (no Plain) ☐ reject ws:// + pin/mTLS ☐ signed channel-bound handshake ☐ node_id from anchored key ☐ version checked on receive ☐ roster admission ☐ S/Kademlia puzzle + α disjoint ☐ WS size cap ☐ bounded buffer ☐ per-IP conn/rate limits ☐ poison recovery ☐ safe-iroh gate met before wiring

**proto-crypto:** ☐ fips_kat (ACVP) ☐ wycheproof ☐ dudect |t|<4.5 ☐ mutation-CI ☐ structural NO-COURIER scan ☐ X-Wing + composite vectors

**build-integrity:** ☐ workspace + committed Cargo.lock + --locked ☐ wasmparser empty-import gate (release, fail-closed) + RED fixture ☐ reproducible build (identical hash) ☐ deny.toml + cargo-audit, fetch-then-check ☐ ban openssl-sys/native-tls ☐ KAT provenance policy + fix fabricated vectors ☐ property-gate structural scan ☐ minimal real reloop/ ☐ kernel/cli built-or-deleted ☐ doc-truth scan

---

## 6. What "100%" means (and honest caveats)

- **100% = every RED test above fails on today's tree and passes only after the fix**, and every GREEN gate is an **external** property. Self-consistency is never sufficient.
- **Two blocking prerequisites** gate the "post-quantum" claim: the **entropy source (F1)** and the **ML-DSA-65 FIPS fix + ACVP (F2)**. Until both are green, treat the **classical (Ed25519) leg as the only load-bearing signature** and do not label anything "post-quantum."
- **A confidential channel is necessary but not sufficient:** proto-wire's TLS does not fix proto-cap's trust model. The W-line is not "secure" until F3/F4/F7 land.
- **Red lines held:** no courier/agent scoring (structural scan), hybrid-only-until-audit (structurally mandatory), and — per `ARCHITECTURE.md:75` — no serde on the signed path.
- **Honest deletions count as progress:** where `kernel/`/`cli/` have no consumer, deleting the README claim is the correct path to 100%, not building vapor. A doc-truth scan enforces "every referenced path exists."

*Standards index: FIPS 203/204/202 · RFC 8032/9106/8439/8949/8785/7250/5705/9266/8446/2693/4303 · NIST SP 800-90A/B/C · draft-connolly-cfrg-xwing-kem · draft-ietf-lamps-pq-composite-sigs · draft-ietf-tls-ecdhe-mlkem · UCAN 1.0 · Noise Protocol Framework · S/Kademlia · dudect (eprint 2016/1123) · NIST ACVP-Server · project-wycheproof.*
