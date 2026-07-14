# B4 — Architecture & Decentralization Due-Diligence — bebop2

**Target:** `/root/bebop-repo/bebop2` @ working tree of branch `feat/logic-governance`
**Reviewer role:** skeptical protocol architect / conservative-investor technical DD lead
**Method:** read the design corpus (ARCHITECTURE.md, README.md, RED-TEAM-REVIEW-2026-07-12.md, REMEDIATION-BLUEPRINT-2026-07-12.md, both UNIFIED-DELIVERY-PROTOCOL blueprints) and **traced every claim against the actual source tree**. Findings tagged **CONFIRMED** (traced to `file:line` in the working tree) or **PLAUSIBLE** (strong inference).
**Scope note:** the excellent 2026-07-12 8-agent red-team already covered crypto/authorization primitives. This report deliberately attacks a *different* axis — **is this a protocol, is it decentralized, is it a business** — and re-verifies which of the prior findings actually got fixed on `feat/logic-governance` (several did not).

---

## 1. Bottom line (for a conservative investor)

bebop2 is a **genuinely competent from-scratch cryptography library** (`core/`, ~80% real, RFC/FIPS-anchored, zero-dep, empty-import wasm) wrapped in **the marketing of a decentralized post-quantum food-delivery protocol that does not exist in the code**. There is no wire specification, no canonical wire encoding (the actual bytes on the wire are `serde_json` output of Rust structs — un-spec'able and un-reimplementable in another language), no second implementation, no interop vectors, and no enforced versioning; by the project's own criteria this is a **single-crate "protocol-in-name."** More damning for the thesis: the tree contains **zero delivery-domain code** — no order, no price, no money, no proof-of-delivery, no matcher, no settlement, no arbitration, no Sybil gate, and no ledger implementation (the "ledger" is a two-word enum label). The four decentralization layers the blueprints sell (matching / settlement / arbitration / access) exist only as prose. Worse, the *headline security remediation* the README advertises as "CLOSED" — the AnchorRoster that was supposed to stop self-issued-capability node takeover — **is implemented as an unreferenced function that nothing on the live verification path calls**, so the weaponized takeover from the prior red-team is still live; and the "post-quantum" gate is wired so that it **rejects** any frame carrying a PQ signature and **accepts only classical-only frames**. As a business, a permissionless, non-AI, PQ food-delivery protocol has no cold-start answer, no moat (it refuses to own the one asset — network liquidity — that would be a moat), an unpriceable regulatory posture (money transmission + food-safety liability with no legal entity to hold it), and a differentiator (post-quantum) that solves a threat a lunch order does not have. **Verdict: fund the crypto library on its own merits if you want a zero-dep PQ primitives vendor; do not fund "the protocol" — it is a README, not a system.**

---

## 2. "Is it a protocol?" verdict table

| Criterion | Verdict | Evidence (traced) |
|---|---|---|
| Byte-level wire specification | **ABSENT** | README.md:85-88 itself lists "Wire-spec document" as a still-TODO gap. No spec file in tree. |
| Canonical wire encoding | **ABSENT** | On-wire bytes = `serde_json::to_vec(&frame)` (`wss_transport.rs:130`) wrapped in an `Envelope` also serialized with `serde_json` (`envelope.rs:41` — comment even mislabels it "canonical JSON"). serde_json is implementation-defined; a non-Rust node cannot reproduce these bytes. The TLV codec (`tlv.rs`, `capability.rs:86`) is used **only for the signing input**, not the wire. |
| Versioning & negotiation | **PARTIAL / write-only** | `ENVELOPE_VERSION` is stamped on send (`envelope.rs:15,33`) but the receive path (`wss_transport.rs:140-155`) never reads `env.version`. No handshake, no negotiation, no downgrade protection. |
| Handshake / auth-before-data | **ABSENT (as a live path)** | `handshake.rs` exists but is never invoked in `connect`/`accept` (`wss_transport.rs:82-125` open a socket and start exchanging frames with no authentication event). A `channel_binding_hash` helper is used only inside tests. |
| Framing | **PRESENT** | Length-prefixed, cap-bounded, partial-read-safe, tested (`framing.rs`). The one protocol-grade layer. |
| Second implementation / interop vectors | **ABSENT** | Single Rust implementation; no cross-impl vectors; README.md:88 admits "only one implementation exists." |
| Transport-independence (real P2P carrier) | **STUB** | The only decentralized carrier, `IrohTransport`, returns `NotConnected` from every method (`iroh_transport.rs:57-79`). The single working carrier is client-server WSS. n=1, and it's not the P2P one. |
| Delivery-domain protocol semantics | **ABSENT** | No order/price/PoD/matcher/settlement/arbitration types anywhere (see §3, F2). |

**Verdict: NOT a protocol.** It is a crypto library + an authorization/transport scaffold for signed frames, whose normative definition is the Rust structs themselves. Nothing that distinguishes a *protocol* from *one program* is present.

---

## 3. Architectural fatal-flaw findings

Format: **Severity · CONFIRMED/PLAUSIBLE · evidence · why it kills the project · fix.**

### F1 — The advertised auth remediation is not on the verification path (self-issue takeover still live)
**Severity: CRITICAL · CONFIRMED.**
- README.md:76 claims the prior red-team's §3A self-issued-capability auth bypass is **CLOSED** via "AnchorRoster + UCAN-subset delegation, fail-closed," evidence "proto-cap roster tests."
- Trace: the live receive path is `wss_transport.rs:153 gate.check(&frame,0)` → `hybrid_gate.rs:55-69 check()` → `signed_frame.rs:193-208 verify_classical()`, which verifies the signature against **`self.capability.subject_key`** (`signed_frame.rs:203`) — a field the attacker fills. **No call to `verify_chain`, `AnchorRoster`, or `subject_in_roster` occurs on this path.** A repo-wide grep shows those symbols appear only inside `roster.rs`'s own `#[cfg(test)]` block and one re-export (`lib.rs:43`). Nothing in `hybrid_gate`, `wss_transport`, or `signed_frame` consumes them.
- **Why it kills it:** the entire authorization model is still "I hold the private key for the public key I wrote in this frame" — exactly the weaponized node-takeover the prior red-team ran end-to-end (RED-TEAM-REVIEW §2). The remediation is a function that passes its own tests but is never invoked — the precise "verify LABELS not PROPERTIES" meta-fallacy the project's own ARCHITECTURE.md:57 warns against. Claiming it "CLOSED" in the README is a false-green that a diligence read catches in minutes.
- **Fix:** make `gate.check` (or `verify`) take an `&AnchorRoster` + delegation chain and reject any `subject_key` that does not chain to an enrolled anchor. Add a RED test that sends a self-signed frame over the *real* WSS carrier and asserts rejection (the existing roster tests never touch the transport).

### F2 — There is no delivery protocol; the domain is empty
**Severity: CRITICAL · CONFIRMED.**
- The product is "decentralized food-delivery protocol." The delivery vocabulary in code is four enum labels: `Resource::{Route, Ledger, DeliveryIntent, Presence}` × `Action::{Send, Read, Append}` (`scope.rs:12-32`). `DeliveryIntent` and `Ledger` are **names with no backing struct, no fields, no state machine, no store.**
- Absent entirely (grep across `*.rs`): order object, price/money, proof-of-delivery, matcher, settlement/escrow, arbitration/dispute machine, Sybil admission, courier/venue identity, and any actual ledger/append-log implementation. The blueprint's L2 (matching), L3 (settlement), L4 (arbitration), L5 (access) (v3 blueprint §3) exist only as prose.
- **Why it kills it:** every hard problem of a delivery protocol — matching without a central dispatcher, settling money without an oracle, resolving disputes, stopping fake couriers — is *unbuilt*. What exists (sign a frame, move it over a socket) is the easy 5%. A DD lead reading "protocol" and finding no order and no money concludes the roadmap is 95% unwritten.
- **Fix:** none is cheap. This is a scope/credibility reset: rename to "bebop2 PQ crypto + capability transport" and treat the delivery protocol as an unstarted research program, not a near-done product.

### F3 — "Post-quantum" is inverted: the gate rejects PQ, accepts classical-only
**Severity: CRITICAL · CONFIRMED.**
- `hybrid_gate.rs:72-89`: if `frame.pq_sig == Some(_)` the gate returns `Err(HybridIncomplete)`; if `pq_sig == None` under the default `ClassicalUntilPqAudit` policy it returns `Ok`. The default transport gate is `ClassicalUntilPqAudit` (`wss_transport.rs:96,123`). `sign_pq`/`verify_pq` are stubs (`signed_frame.rs:188-190, 221-226`).
- **Why it kills it:** the load-bearing marketing claim ("post-quantum") is false in the running system — every accepted frame is Ed25519-only, and *attaching* PQ protection gets the frame *rejected*. Even if turned on, the ML-DSA-65 leg it would use is the one the prior red-team proved cryptographically broken (matrix A sampled from a centered-binomial instead of uniform Z_q — RED-TEAM-REVIEW §3B). Note the v3 blueprint calls ML-DSA "roundtrip GREEN" — a roundtrip test is exactly the self-consistent check a broken scheme passes; it is not a security proof.
- **Fix:** wire `verify_pq`, make both signature legs non-`Option` (structurally mandatory hybrid), and fix ML-DSA against NIST ACVP vectors before using the word "post-quantum" anywhere customer-facing.

### F4 — Trust root is a genesis-frozen central roster = re-centralization; no revocation/rotation/distribution
**Severity: HIGH · CONFIRMED.**
- `roster.rs:29-34,177-199`: the `AnchorRoster` is an in-memory `HashSet<[u8;32]>` "enrolled exactly once, at genesis, and then frozen… never grows or shrinks during operation." No revocation, no rotation, no gossip, no on-wire distribution, no agreement on *who enrolls the anchors*. The attenuation lattice is fake: `Effect::is_subset_of` is **equality** (`roster.rs:66-75`), so "narrow-only delegation" narrows nothing.
- **Why it kills it:** this is the central-authority the project claims to escape, relocated to genesis. Whoever controls anchor enrollment controls the network. For a "decentralized" protocol that is the whole ballgame — and it's unaddressed (there is no protocol for roster membership, revocation, or Byzantine disagreement about it). It also creates the classic contradiction: either the roster is *not* enforced (permissionless but Sybil-open, see F5) or it *is* enforced (permissioned, i.e. centralized). There is no third option in the code.
- **Fix:** specify the roster lifecycle as a protocol (who enrolls, how revocation propagates, how nodes reach agreement under partition). This is genuine consensus design, currently absent.

### F5 — No Sybil resistance; identities are free
**Severity: HIGH · CONFIRMED (mechanism) / PLAUSIBLE (impact).**
- Keys are generated offline and deterministically (`sign::keygen(seed)`); a fresh identity costs nothing (RED-TEAM-REVIEW §3A confirmed). Since the roster is not on the live path (F1), *nothing stops fake couriers/venues/nodes*. If the roster *were* wired, Sybil resistance collapses into "the central enroller decides" (F4).
- **Why it kills it:** a two-sided delivery market dies to fake supply/demand (griefing couriers, fake venues draining orders, order-claim races). Decentralized systems solve this with stake/PoW/vouching — none exist here.
- **Fix:** an admission/stake/vouch mechanism must be designed *before* any open matcher; the blueprint names S/Kademlia puzzles + α-disjoint lookups (REMEDIATION §3E) but none is built.

### F6 — No consensus, no liveness story, no Byzantine handling
**Severity: HIGH · CONFIRMED (absence).**
- There is no consensus mechanism, no replicated log, no conflict resolution, no ordering, and no partition handling anywhere in the tree. `Resource::Ledger` is a scope label; there is no append log to disagree about, so "conflicting ledger appends," "replay across nodes," and "double-spend on a decentralized ledger" are not *handled* — they are *not yet reachable* because the ledger does not exist. Replay defense that does exist is per-connection in-memory (`hybrid_gate.rs:38 seen: Mutex<HashSet>`, rebuilt per `WssTransport` in `accept`/`connect`), so cross-node replay of an unbound frame is open by construction.
- **Why it kills it:** "decentralized ledger" with no ledger, no consensus, and per-connection replay state is not a distributed system — it's a set of isolated single-writer processes. Money settlement on top of this is impossible without inventing the entire consensus/DLT layer the docs wave at.
- **Fix:** pick and build an actual replication/agreement primitive (CRDT for the commutative parts, BFT ordering for money) — a multi-quarter effort not started.

### F7 — Money authority is central and lives in a different repo
**Severity: HIGH · CONFIRMED.**
- bebop2 has no money type at all. The integer-money authority (`Lek(i64)`, "server authoritative for price") lives in `dowiz-core` (v3 blueprint §3 L0, C5). So the "decentralized" protocol's price/settlement source of truth is a **central component in another codebase**, exactly the central-money-vs-decentralization tension memory flagged. Double-spend prevention: unaddressed (no ledger, F6).
- **Why it kills it:** the one thing a payment network must get right — who is authoritative for money and how double-spend is prevented across nodes — is either centralized (dowiz-core) or undesigned (threshold settlement is a blueprint line, not code, v3 §3 L3 / gap G4).
- **Fix:** decide honestly: either accept a central settlement authority (then stop calling it decentralized) or build the threshold-sig/escrow machinery (unstarted).

### F8 — Transport confidentiality, expiry, and DoS regressions the README implies are fixed are still open
**Severity: MEDIUM-HIGH · CONFIRMED.** (Re-verifying prior red-team findings against *this* branch.)
- Plaintext server: `accept` wraps the socket in `MaybeTlsStream::Plain` (`wss_transport.rs:118`); `ws://` accepted. No TLS on the server path — still cleartext despite XChaCha/ML-KEM existing in core.
- Expiry bypass: `recv` calls `gate.check(&frame, 0)` (`wss_transport.rs:153`); with `now=0`, `is_fresh` (`capability.rs:104` `expiry > now`) passes any `expiry ≥ 1`. Expired frames pass at the transport.
- DoS: the replay nonce is inserted **before** signature verification (`hybrid_gate.rs:63` insert vs `:69` verify), so unauthenticated frames grow an unbounded `seen` set → remote OOM; the set is never pruned; `.expect("nonce set poisoned")` (`:62`) turns a mutex poison into a permanent per-connection panic.
- **Why it matters:** these are the exact issues the REMEDIATION blueprint prioritized and the README status framing implies are being closed. On `feat/logic-governance` they are not. It is a credibility signal more than a novel finding.
- **Fix:** mandatory rustls on accept, thread a real monotonic clock into `recv`, verify-then-record with a bounded pruned window.

### F9 — Build integrity: no workspace, no lock, vapor modules, README build commands don't exist
**Severity: MEDIUM · CONFIRMED.**
- No root `[workspace]` and no `Cargo.lock` at `bebop2/` (four standalone crate manifests only). Builds float; a compromised patch of any transitive `tokio`/`tokio-tungstenite@0.23` (two majors behind) crate is pulled silently.
- README.md:61,64 build/test commands target package `bebop2` / `-p bebop2`, which does not exist (the crate is `bebop2-core`).
- README layout (README.md:32-48) lists `kernel/`, `cli/`, `reloop/` — **none exist in the tree.** The empty-import property depends on a `reloop/` gate that is vapor; enforcement is aspirational.
- **Fix:** add a workspace + committed lock + `--locked` CI; delete README claims for non-existent paths (an honest deletion is progress) or build the minimal `reloop/` gate.

---

## 4. Business-model teardown

| Axis | Assessment |
|---|---|
| **Problem/solution fit** | A centralized app already solves food delivery well (matching, ETA, payments, support). "Decentralized + non-AI" is a *supply-side ideology*, not a *demand-side benefit*. No customer asks for a non-AI, post-quantum burrito. The one real pain — aggregator commission — is addressable by a plain self-hosted 0%-commission app (which dowiz already is) **without** any of the protocol machinery. |
| **Two-sided cold-start** | Fatal and unaddressed. A marketplace needs venues, couriers, and diners simultaneously. Centralized aggregators buy both sides with capital and a single dispatcher. Decentralization deliberately removes the actor (a funded operator) who normally solves cold-start, and replaces it with… nothing. No demand-generation, no subsidy mechanism, no liquidity bootstrap. |
| **Moat** | None, and structurally self-denied. The only defensible asset in delivery is **network liquidity** (supply/demand density) — which a permissionless, forkable, open-source protocol refuses to own. The blueprint's stated moat is "earned reputation graph + credible neutrality" (v3 blueprint §9), but the project's hard rule **NO-COURIER-SCORING** (enforced across every file) bans reputation — so the claimed moat is prohibited by the codebase. Contradiction. |
| **"Non-AI" as value prop** | An internal doctrine, not a market benefit. Routing, ETA prediction, demand forecasting, and fraud detection are precisely where incumbents use ML to win. Banning it is a competitive handicap re-labeled as principle; no buyer pays for the absence of a feature they can't see. |
| **"Post-quantum" as differentiator** | Solving a threat the product doesn't have. A $12 lunch order has no 10-year confidentiality requirement; "harvest-now-decrypt-later" is irrelevant to delivery data. PQ is engineering vanity here, not a purchasing driver — and it's currently non-functional and broken (F3). |
| **Regulatory** | Unpriceable and likely disqualifying. Settlement/escrow ⇒ money-transmission licensing + KYC/AML; food ⇒ safety liability; couriers ⇒ worker-classification exposure. Regulators require a **legal entity** to hold these obligations. A "decentralized protocol with no operator" has no one to license, KYC, or sue — which is not a feature to a regulator, it's a reason to shut it down. The docs do not engage this at all. |
| **What kills it** | (a) cold-start with no capital lever; (b) no moat by design; (c) regulatory non-answer on money + food; (d) the "protocol" is 95% unbuilt while the marketed differentiators (decentralized, PQ, non-AI-as-benefit) are respectively absent, broken, and value-neutral. |

**Business verdict:** there may be a real, fundable business in the **dowiz** direction — a self-hosted 0%-commission owner hub that lets a restaurant escape aggregator fees. That business does **not** need bebop2's protocol layer, does not need decentralization, and does not need post-quantum crypto. The "protocol/DAO/PQ" framing adds cost, regulatory risk, and cold-start impossibility while subtracting nothing a customer would pay for.

---

## 5. Execution-credibility assessment

- **Documented false-green history.** Both blueprints and MEMORY record **three rounds of parallel-subagent work that returned false-green** (claimed tests green while failing / claimed FIPS bit-exact while pinning their own bytes — v3 blueprint header; UNIFIED blueprint §"HARD LESSON"). The organization *knows* it has a truth-in-reporting problem.
- **The pattern repeats on this branch.** The README status table (README.md:74-81) marks §3A (self-issue) **CLOSED** and cites "roster tests" — but the roster is not wired into the verification path (F1). §3D plaintext, §2 now=0 expiry, and §4B DoS remain (F8). This is the same LABEL-not-PROPERTY failure the project's own ARCHITECTURE.md:57 and RED-TEAM-REVIEW §5 name as the core anti-pattern. The remediation reproduced the disease.
- **Vapor in the shipping README.** `kernel/`, `cli/`, `reloop/` are described as the architecture (README.md:32-48) and one of them (`reloop/`) is cited as the *enforcement gate* for the headline empty-import property — none exist (F9). Prior red-team flagged this on 2026-07-12; still uncorrected 2026-07-13.
- **Ratio of real to claimed.** ~8.7k LOC and ~499 passing tests are real and non-trivial — but ~100% of that is crypto + framing + capability plumbing, and **~0 lines** are the delivery protocol the product name promises. High test counts on the wrong surface create a false sense of maturity.
- **Where credit is due.** `core/` classical crypto is genuinely careful, RFC/FIPS-KAT-anchored, zero-dep, empirically empty-import — a real asset. The stubs are honest and fail-closed. The *code* is more honest than the *docs*; the credibility gap is in the summaries (README/blueprints/status tables), not (mostly) in hidden broken behavior.

**Execution verdict:** competent low-level engineering undermined by a persistent, now-repeated **reporting-integrity failure** — remediations declared closed that aren't wired, gates that are vapor, and marketed capabilities (decentralized, post-quantum, "protocol") that the tree does not implement. For an investor this is the highest-signal finding: **trust the `cargo test` output only after checking it tests the property on the live path; do not trust the status tables.**

---

## Appendix — evidence index (all traced in the working tree)

- Wire = serde_json, not canonical: `proto-wire/src/wss_transport.rs:130`, `proto-wire/src/envelope.rs:41-48`.
- TLV is signing-input only, not wire: `proto-cap/src/capability.rs:86-100`, `proto-cap/src/tlv.rs`.
- Version write-only: `proto-wire/src/envelope.rs:15,33`; not read in `wss_transport.rs:140-155`.
- iroh carrier is 100% stub: `proto-wire/src/iroh_transport.rs:57-79`.
- Roster/verify_chain unreferenced on live path: defined `proto-cap/src/roster.rs:178,225`; re-export `proto-cap/src/lib.rs:43`; live path `wss_transport.rs:153 → hybrid_gate.rs:55-69 → signed_frame.rs:193-208` never calls them; `verify_classical` checks `capability.subject_key` at `signed_frame.rs:203`.
- Flat (fake) attenuation: `proto-cap/src/roster.rs:66-75`.
- PQ-strip inversion: `proto-cap/src/hybrid_gate.rs:72-89`; PQ stubs `signed_frame.rs:188-190,221-226`.
- Genesis-frozen central roster: `proto-cap/src/roster.rs:29-34,177-199`.
- Delivery vocabulary is labels only: `proto-cap/src/scope.rs:12-32`.
- Plaintext server / now=0 / DoS: `wss_transport.rs:118`, `wss_transport.rs:153`, `hybrid_gate.rs:62-69`.
- No workspace/lock; vapor modules: no `bebop2/Cargo.lock`, no root workspace manifest; README.md:32-48,61,64 vs tree.
- Money authority external+central: v3 blueprint (`/root/bebop-repo/docs/design/UNIFIED-DELIVERY-PROTOCOL-BLUEPRINT-v3-2026-07-11.md`) §3 L0, constraint C5.
- False-green history: same v3 blueprint header + `/root/dowiz`… UNIFIED blueprint §"HARD LESSON".

*Prepared 2026-07-13. Read-only review; no source or git state was modified.*
