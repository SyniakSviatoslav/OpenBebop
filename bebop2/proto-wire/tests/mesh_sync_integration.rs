//! Wave 5 (MESH cohesion) — end-to-end integration test.
//!
//! Proves that `bebop2-core`'s local anti-entropy ([`bebop2_core::anti_entropy`]
//! `digest`/`diff`/`apply_pull` over an [`bebop2_core::event_log::EventLog`]) can
//! be wired over the REAL QUIC transport ([`bebop_proto_wire::iroh_transport`],
//! the iroh/MESH-09 quinn carrier) using the existing MESH-07 sync protocol
//! ([`bebop_proto_wire::sync_pull`] `SyncFrame` wire codec) to achieve
//! *convergent, bidirectional* mesh sync.
//!
//! It is an INTEGRATION TEST only — it adds no source; `sync_pull.rs`,
//! `iroh_transport.rs`, `anti_entropy.rs`, and `event_log.rs` are untouched.
//!
//! # How the pieces fit
//! * `EventLog` is the append-only hash-chain source of truth. Two nodes
//!   converge when their `root_hash()` are equal.
//! * `anti_entropy::digest`/`diff` compute the exact suffix one node is missing
//!   from the other (the "behind" case — forks are detected but not merged, by
//!   design; see `anti_entropy.rs`).
//! * `SyncFrame` (MESH-07) packages each missing event as a content-addressed,
//!   signed, wire-encodable unit. `to_wire_bytes`/`from_wire_bytes` is the
//!   canonical binary codec.
//! * `QuicTransport` (MESH-09) is the REAL quinn/QUIC carrier. Its `recv`
//!   enforces the `RequireBoth` hybrid gate, so every frame we ship is wrapped
//!   in a properly anchored, double-signed `SignedFrame` (real Ed25519 +
//!   real ML-DSA-65 + anchor-rooted delegation chain) — exactly like the
//!   existing round-trip tests in `iroh_transport.rs`.
//!
//! Because `QuicTransport::send` finishes the bi-stream after each frame, we
//! ship ONE batch (all missing `SyncFrame`s length-prefixed into a single
//! payload) per QUIC connection, and run one connection per sync direction.
//! Idempotency is provided by content-id dedup in a `MerkleLog`: re-delivering
//! an already-folded event is a no-op (no double `apply_pull`).
//!
//! Run with: `cargo test -p bebop-proto-wire --features insecure-test --test mesh_sync_integration`

use std::sync::{Arc, LazyLock};

use bebop2_core::anti_entropy::{apply_pull, diff, digest};
use bebop2_core::event_log::EventLog;
use bebop2_core::pq_dsa::{derive_pq_seed, keygen_derivable};
use bebop2_core::sign::keygen;
use bebop_proto_cap::roster::{AnchorRoster, Delegation, Effect};
use bebop_proto_cap::scope::{Action, Resource, Scope};
use bebop_proto_cap::{Capability, SignedFrame};
use bebop_proto_wire::discovery::{GossipAgent, PeerDirectory, SignedEndpoint};
use bebop_proto_wire::iroh_transport::{QuicEndpoint, QuicTransport};
use bebop_proto_wire::sync_pull::{MerkleLog, SyncFrame, SyncScope};
use bebop_proto_wire::Transport;
use tokio::sync::Mutex;

/// Serialize the QUIC tests: they grab ephemeral UDP ports, and running them
/// concurrently can recycle a just-released port before the prior endpoint fully
/// unbinds (EADDRINUSE). One at a time avoids the race (mirrors the lock in
/// `iroh_transport.rs`'s own tests).
static QUIC_PORT_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

/// Capability expiry far in the future (the live gate computes `now` from the
/// wall clock, so any small test value would still be fresh).
const EXPIRY: u64 = 9_999_999_999;

/// Deterministic Ed25519 key from a single seed byte (test_keygen feature).
fn actor(seed_byte: u8) -> ([u8; 32], [u8; 32]) {
    let seed = [seed_byte; 32];
    let pk = keygen(&seed).1;
    (seed, pk)
}

/// Deterministic canonical event payloads (the shared master chain content).
fn master_payloads(n: usize) -> Vec<Vec<u8>> {
    (0..n).map(|i| format!("mesh-event-{i}").into_bytes()).collect()
}

/// Build an `EventLog` from a slice of payloads (in order).
fn seed_log(payloads: &[Vec<u8>]) -> EventLog<()> {
    let mut log = EventLog::new();
    for p in payloads {
        log.append(p);
    }
    log
}

/// Anchor -> actor delegation chain carrying the mesh `Sync::Pull` scope. The
/// chain is reused across every frame (the hybrid gate records only the
/// *capability* nonce, not the chain, so reusing the chain is safe).
fn build_chain(
    anchor_seed: &[u8; 32],
    anchor_pk: &[u8; 32],
    actor_pk: &[u8; 32],
) -> Vec<Delegation> {
    let link = Delegation::sign(
        *anchor_pk,
        *actor_pk,
        Scope::single(Resource::Sync, Action::Pull),
        Effect::single(Resource::Sync, Action::Pull),
        EXPIRY,
        [1u8; 8],
        anchor_seed,
    )
    .expect("anchor delegation must sign");
    vec![link]
}

/// Wrap a batch of `SyncFrame` wire-bytes into ONE hybrid-signed `SignedFrame`
/// that the QUIC carrier's `RequireBoth` gate will accept: real Ed25519
/// (classical) + real ML-DSA-65 (PQ, derived from the same master seed) over a
/// capability scoped to `Sync::Pull`, carrying an anchor-rooted delegation chain.
fn wrap_batch(
    actor_seed: &[u8; 32],
    actor_pk: &[u8; 32],
    batch: Vec<u8>,
    chain: &[Delegation],
    nonce: u64,
) -> SignedFrame {
    let pq_seed = derive_pq_seed(actor_seed);
    let (pq_pk, pq_sk) = keygen_derivable(&pq_seed);
    let cap = Capability::new_hybrid(
        *actor_pk,
        pq_pk.bytes.clone(),
        Resource::Sync,
        Action::Pull,
        nonce.to_le_bytes(),
        EXPIRY,
    );
    let mut f = SignedFrame::new(cap, batch);
    f.sign_classical(actor_seed).expect("classical sign");
    f.sign_pq(
        &pq_sk.bytes.clone().try_into().expect("pq sk width"),
        &[0u8; 32],
    )
    .expect("pq sign");
    f.delegation_chain = chain.to_vec();
    f
}

/// Length-prefixed concatenation of `SyncFrame` wire images (one QUIC frame can
/// carry a whole delta batch).
fn encode_batch(frames: &[SyncFrame]) -> Vec<u8> {
    let mut out = Vec::new();
    for f in frames {
        let w = f.to_wire_bytes();
        out.extend_from_slice(&(w.len() as u32).to_le_bytes());
        out.extend_from_slice(&w);
    }
    out
}

/// Inverse of [`encode_batch`].
fn decode_batch(bytes: &[u8]) -> Vec<SyncFrame> {
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < bytes.len() {
        let len =
            u32::from_le_bytes(bytes[i..i + 4].try_into().expect("len prefix")) as usize;
        i += 4;
        let w = &bytes[i..i + len];
        out.push(SyncFrame::from_wire_bytes(w).expect("decode sync frame"));
        i += len;
    }
    out
}

/// Establish one real QUIC connection and ship `signed` from the client to the
/// server, returning the server-side received (and already hybrid-gate-verified)
/// `SignedFrame`. Both endpoints enroll `roster` so the anchor chain verifies.
async fn quic_ship(addr: String, signed: SignedFrame, roster: AnchorRoster) -> SignedFrame {
    let (tx, rx) = tokio::sync::oneshot::channel();
    let server = tokio::spawn({
        let addr = addr.clone();
        let roster = roster.clone();
        async move {
            let _ = tx.send(());
            let ep = QuicEndpoint::Bind(addr);
            let mut t = QuicTransport::accept(&ep)
                .await
                .expect("server accept")
                .with_roster(roster);
            t.recv().await.expect("server recv")
        }
    });
    rx.await.unwrap();
    let mut client = QuicTransport::connect(&QuicEndpoint::Dial(addr))
        .await
        .expect("client connect")
        .with_roster(roster);
    client.send(signed).await.expect("client send");
    server.await.unwrap()
}

/// Grab a free loopback UDP port (QUIC rides UDP).
async fn free_port() -> String {
    let sock = tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let addr = sock.local_addr().unwrap().to_string();
    drop(sock);
    addr
}

/// Fold a received batch into `to_log` via `apply_pull`, using `to_folded`
/// (a `MerkleLog` of content-ids) for content-addressed idempotent dedup.
/// Returns `(added, dup)`.
fn fold_batch(to_log: &mut EventLog<()>, to_folded: &mut MerkleLog, batch: &[u8]) -> (usize, usize) {
    let frames = decode_batch(batch);
    let mut new_events: Vec<(u64, Vec<u8>)> = Vec::new();
    let mut dup = 0usize;
    for sf in &frames {
        sf.verify().expect("received sync frame must verify (scope+sig+content_id)");
        if to_folded.contains(&sf.content_id) {
            dup += 1; // content-addressed no-op
            continue;
        }
        new_events.push((sf.seq, sf.payload.clone()));
    }
    if !new_events.is_empty() {
        let refs: Vec<(u64, &[u8])> =
            new_events.iter().map(|(s, p)| (*s, p.as_slice())).collect();
        // `apply_pull` requires the local log to be a clean prefix of the remote
        // (the "behind" case); the missing events arrive in ascending seq order,
        // so each `seq` equals the next `log.len()` — no forks.
        apply_pull(to_log, &refs).expect("apply_pull must succeed (clean prefix)");
        for sf in &frames {
            to_folded.add(sf.content_id);
        }
    }
    (new_events.len(), dup)
}

/// Build the `SyncFrame`s for a missing suffix (collected via `EventLog::replay`)
/// and ship them as one QUIC batch from sender -> receiver, folding on receipt.
async fn sync_missing(
    sender_log: &EventLog<()>,
    receiver_log: &mut EventLog<()>,
    receiver_folded: &mut MerkleLog,
    plan: &bebop2_core::anti_entropy::SyncPlan,
    actor_seed: &[u8; 32],
    actor_pk: &[u8; 32],
    chain: &[Delegation],
    roster: &AnchorRoster,
    nonce: &mut u64,
) {
    let missing: Vec<(u64, Vec<u8>)> = sender_log
        .replay(plan.pull_from)
        .map(|(s, p)| (s, p.to_vec()))
        .collect();
    assert_eq!(missing.len(), plan.pull_len, "replay must yield the planned suffix");
    let sync_frames: Vec<SyncFrame> = missing
        .iter()
        .map(|(s, p)| {
            SyncFrame::sign(SyncScope::pull(), [0u8; 32], *actor_pk, *s, p.clone(), actor_seed)
        })
        .collect();
    let batch = encode_batch(&sync_frames);
    *nonce += 1;
    let signed = wrap_batch(actor_seed, actor_pk, batch, chain, *nonce);
    let addr = free_port().await;
    let received = quic_ship(addr, signed, roster.clone()).await;
    fold_batch(receiver_log, receiver_folded, &received.payload);
}

// ── TEST 1: forward convergence (A's events propagate to B, B converges) ──

#[cfg(feature = "insecure-tls")]
#[tokio::test]
async fn mesh_sync_converges_forward() {
    let _lock = QUIC_PORT_LOCK.lock().await;

    // Shared master chain of 10 events. A holds all 10; B holds a clean prefix (4).
    let master = master_payloads(10);
    let mut a = seed_log(&master[0..10]);
    let mut b = seed_log(&master[0..4]);
    let mut b_folded = MerkleLog::new();

    let (actor_seed, actor_pk) = actor(2);
    let (anchor_seed, anchor_pk) = actor(1);
    let chain = build_chain(&anchor_seed, &anchor_pk, &actor_pk);
    let mut roster = AnchorRoster::new();
    roster.enroll(&anchor_pk);
    let mut nonce = 0u64;

    // anti-entropy: what is B (local) missing from A (remote)?
    let plan = diff(&digest(&b), &digest(&a));
    assert_eq!(plan.pull_from, 4, "B is missing from seq 4");
    assert_eq!(plan.pull_len, 6, "B must pull exactly 6 events");

    sync_missing(
        &a,
        &mut b,
        &mut b_folded,
        &plan,
        &actor_seed,
        &actor_pk,
        &chain,
        &roster,
        &mut nonce,
    )
    .await;

    // Convergence: B's EventLog now equals A's.
    assert_eq!(b.len(), 10, "B folded exactly the missing suffix");
    assert!(b.verify().is_ok(), "B's chain verifies after the pull");
    assert_eq!(
        b.root_hash(),
        a.root_hash(),
        "B's root_hash must equal A's (convergence)"
    );
}

// ── TEST 2: bidirectional convergence (B's distinct events also reach A) ──

#[cfg(feature = "insecure-tls")]
#[tokio::test]
async fn mesh_sync_bidirectional() {
    let _lock = QUIC_PORT_LOCK.lock().await;

    // Master chain of 12 events.
    let master = master_payloads(12);
    // Round 1 seed: A has [0..9], B has [0..5] (B is behind A by 4).
    let mut a = seed_log(&master[0..9]);
    let mut b = seed_log(&master[0..5]);
    let mut a_folded = MerkleLog::new();
    let mut b_folded = MerkleLog::new();

    let (actor_seed, actor_pk) = actor(2);
    let (anchor_seed, anchor_pk) = actor(1);
    let chain = build_chain(&anchor_seed, &anchor_pk, &actor_pk);
    let mut roster = AnchorRoster::new();
    roster.enroll(&anchor_pk);
    let mut nonce = 0u64;

    // ---- Direction 1 (A -> B): A's distinct events [5..9] propagate to B ----
    let plan1 = diff(&digest(&b), &digest(&a));
    assert_eq!(plan1.pull_from, 5);
    assert_eq!(plan1.pull_len, 4);
    sync_missing(
        &a,
        &mut b,
        &mut b_folded,
        &plan1,
        &actor_seed,
        &actor_pk,
        &chain,
        &roster,
        &mut nonce,
    )
    .await;
    assert_eq!(b.len(), 9);
    assert_eq!(b.root_hash(), a.root_hash(), "B converged to A after dir-1");

    // ---- Direction 2 (B -> A): B now authors distinct events [9..12] ----
    for p in &master[9..12] {
        b.append(p);
    }
    let plan2 = diff(&digest(&a), &digest(&b));
    assert_eq!(plan2.pull_from, 9);
    assert_eq!(plan2.pull_len, 3);
    sync_missing(
        &b,
        &mut a,
        &mut a_folded,
        &plan2,
        &actor_seed,
        &actor_pk,
        &chain,
        &roster,
        &mut nonce,
    )
    .await;

    // Both nodes now hold the full master chain -> identical roots.
    assert_eq!(a.len(), 12);
    assert_eq!(b.len(), 12);
    assert!(a.verify().is_ok());
    assert!(b.verify().is_ok());
    assert_eq!(
        a.root_hash(),
        b.root_hash(),
        "both nodes converged to the same root (bidirectional)"
    );
    // And it matches a freshly-built full master chain.
    let full = seed_log(&master[0..12]);
    assert_eq!(a.root_hash(), full.root_hash());
}

// ── TEST 3: idempotent re-delivery (content-addressed dedup, no double-fold) ──

#[cfg(feature = "insecure-tls")]
#[tokio::test]
async fn mesh_sync_idempotent() {
    let _lock = QUIC_PORT_LOCK.lock().await;

    // A holds all 8; B holds a prefix of 3.
    let master = master_payloads(8);
    let mut a = seed_log(&master[0..8]);
    let mut b = seed_log(&master[0..3]);
    let mut b_folded = MerkleLog::new();

    let (actor_seed, actor_pk) = actor(2);
    let (anchor_seed, anchor_pk) = actor(1);
    let chain = build_chain(&anchor_seed, &anchor_pk, &actor_pk);
    let mut roster = AnchorRoster::new();
    roster.enroll(&anchor_pk);
    let mut nonce = 0u64;

    let plan = diff(&digest(&b), &digest(&a));
    assert_eq!(plan.pull_from, 3);
    assert_eq!(plan.pull_len, 5);

    // First delivery (fresh QUIC connection + fresh nonce).
    sync_missing(
        &a,
        &mut b,
        &mut b_folded,
        &plan,
        &actor_seed,
        &actor_pk,
        &chain,
        &roster,
        &mut nonce,
    )
    .await;
    assert_eq!(b.len(), 8);
    assert_eq!(b.root_hash(), a.root_hash(), "first delivery converges");

    // Re-deliver the SAME missing batch (different QUIC connection, different
    // capability nonce, but identical content-ids). Must be a pure no-op.
    let before_len = b.len();
    let before_root = b.root_hash();
    sync_missing(
        &a,
        &mut b,
        &mut b_folded,
        &plan,
        &actor_seed,
        &actor_pk,
        &chain,
        &roster,
        &mut nonce,
    )
    .await;

    assert_eq!(b.len(), before_len, "re-delivery must not change log length");
    assert_eq!(b.root_hash(), before_root, "re-delivery must not change root");
    assert_eq!(b.root_hash(), a.root_hash(), "still converged after re-delivery");

    // Sanity: the dedup set actually saw the dups (no double-fold happened).
    assert_eq!(b_folded.len(), 5, "exactly 5 distinct content-ids folded");
}

/// One DIRECTED anti-entropy pull: `receiver` pulls its missing clean-prefix
/// suffix from `sender` over a single fresh real QUIC connection. A no-op
/// (no connection) when the receiver is already caught up. Reuses [`sync_missing`]
/// so the full hybrid-signed frame + roster path is exercised exactly as the
/// 2-node tests.
async fn directed_pull(
    sender_log: &EventLog<()>,
    receiver_log: &mut EventLog<()>,
    receiver_folded: &mut MerkleLog,
    actor_seed: &[u8; 32],
    actor_pk: &[u8; 32],
    chain: &[Delegation],
    roster: &AnchorRoster,
    nonce: &mut u64,
) {
    let plan = diff(&digest(receiver_log), &digest(sender_log));
    if plan.pull_len == 0 {
        return; // receiver already caught up — no QUIC connection needed
    }
    sync_missing(
        sender_log,
        receiver_log,
        receiver_folded,
        &plan,
        actor_seed,
        actor_pk,
        chain,
        roster,
        nonce,
    )
    .await;
}

// ── TEST 4: 3-node mesh convergence over real QUIC (W11) ──
//
// Prove the sync layer converges an N-node (3) mesh: A, B, C each run their own
// EventLog + anti_entropy over a REAL quinn/QUIC endpoint, and after enough
// all-pairs anti_entropy sweeps all three hold IDENTICAL (length + root_hash)
// state. No single node is seeded with the union; node B is the EMPTY relay and
// is the one that authors the final events, so A and C converge *through* B
// (transitive mesh, not a star) — the minimal non-trivial mesh property.
//
// innovate: real discovery/gossip (MESH-02/03) is out of scope here; this proves
// the *convergence property* of the sync layer over the real transport with N
// endpoints. A fork (two nodes with conflicting seq-0 content) cannot be merged
// by the behind-only `apply_pull` by design — a mesh must therefore share ONE
// canonical ordered master chain, which is exactly what every node converges to.

#[cfg(feature = "insecure-tls")]
#[tokio::test]
async fn mesh_sync_converges_3node() {
    let _lock = QUIC_PORT_LOCK.lock().await;

    // One canonical ordered master chain of 12 events.
    let master = master_payloads(12);
    // Seed: A holds K=4 (a prefix), C holds M=7 DIFFERENT-length prefix, B empty
    // (the relay). All three are clean prefixes of `master` — no forks.
    let mut a = seed_log(&master[0..4]);
    let mut b = seed_log(&master[0..0]);
    let mut c = seed_log(&master[0..7]);
    let mut a_folded = MerkleLog::new();
    let mut b_folded = MerkleLog::new();
    let mut c_folded = MerkleLog::new();

    let (actor_seed, actor_pk) = actor(2);
    let (anchor_seed, anchor_pk) = actor(1);
    let chain = build_chain(&anchor_seed, &anchor_pk, &actor_pk);
    let mut roster = AnchorRoster::new();
    roster.enroll(&anchor_pk);
    let mut nonce = 0u64;

    // One full all-pairs sweep over REAL QUIC (A<->B, B<->C, A<->C, both ways).
    // Each directed pull opens its own loopback QUIC connection + hybrid frame.
    async fn sweep(
        a: &mut EventLog<()>,
        b: &mut EventLog<()>,
        c: &mut EventLog<()>,
        a_folded: &mut MerkleLog,
        b_folded: &mut MerkleLog,
        c_folded: &mut MerkleLog,
        actor_seed: &[u8; 32],
        actor_pk: &[u8; 32],
        chain: &[Delegation],
        roster: &AnchorRoster,
        nonce: &mut u64,
    ) {
        directed_pull(a, b, b_folded, actor_seed, actor_pk, chain, roster, nonce).await;
        directed_pull(b, &mut *a, a_folded, actor_seed, actor_pk, chain, roster, nonce).await;
        directed_pull(b, c, c_folded, actor_seed, actor_pk, chain, roster, nonce).await;
        directed_pull(c, b, b_folded, actor_seed, actor_pk, chain, roster, nonce).await;
        directed_pull(a, c, c_folded, actor_seed, actor_pk, chain, roster, nonce).await;
        directed_pull(c, &mut *a, a_folded, actor_seed, actor_pk, chain, roster, nonce).await;
    }

    // Round 1: propagate the two distinct prefixes through the mesh. B (empty)
    // receives from both A and C; A catches up to C's deeper prefix.
    sweep(
        &mut a, &mut b, &mut c, &mut a_folded, &mut b_folded, &mut c_folded, &actor_seed, &actor_pk,
        &chain, &roster, &mut nonce,
    )
    .await;
    assert_eq!(a.len(), 7, "A caught up to C's prefix via the mesh");
    assert_eq!(b.len(), 7, "B relayed both prefixes up to depth 7");
    assert_eq!(c.len(), 7);
    assert_eq!(a.root_hash(), c.root_hash(), "A and C agree at depth 7");
    assert_eq!(b.root_hash(), c.root_hash(), "B agrees at depth 7");

    // The relay node B authors the FINAL events (e7..e11) on top of the shared
    // prefix. A and C will converge to these ONLY by pulling from B — i.e. they
    // depend on the mesh relay, not a direct A<->C share of the new content.
    for p in &master[7..12] {
        b.append(p);
    }

    // Round 2: B pushes the new tail to A and C. A<->C over the new events is a
    // no-op (neither authored them) — the new content flows through B.
    sweep(
        &mut a, &mut b, &mut c, &mut a_folded, &mut b_folded, &mut c_folded, &actor_seed, &actor_pk,
        &chain, &roster, &mut nonce,
    )
    .await;
    assert_eq!(a.len(), 12, "A converged to full master via B");
    assert_eq!(b.len(), 12);
    assert_eq!(c.len(), 12, "C converged to full master via B");
    assert!(a.verify().is_ok());
    assert!(b.verify().is_ok());
    assert!(c.verify().is_ok());
    assert_eq!(a.root_hash(), b.root_hash(), "all three root_hashes identical");
    assert_eq!(b.root_hash(), c.root_hash(), "all three root_hashes identical");

    // Idempotency: an extra full all-pairs sweep must be a pure no-op.
    let before = (a.len(), b.len(), c.len(), a.root_hash());
    sweep(
        &mut a, &mut b, &mut c, &mut a_folded, &mut b_folded, &mut c_folded, &actor_seed, &actor_pk,
        &chain, &roster, &mut nonce,
    )
    .await;
    assert_eq!(a.len(), before.0, "extra sweep: A length unchanged");
    assert_eq!(b.len(), before.1, "extra sweep: B length unchanged");
    assert_eq!(c.len(), before.2, "extra sweep: C length unchanged");
    assert_eq!(a.root_hash(), before.3, "extra sweep: A root unchanged");
    assert_eq!(a.root_hash(), b.root_hash(), "still converged after extra sweep");
    assert_eq!(b.root_hash(), c.root_hash(), "still converged after extra sweep");

    // And it matches a freshly built full master chain (content-addressed state).
    let full = seed_log(&master[0..12]);
    assert_eq!(a.root_hash(), full.root_hash(), "converged root == canonical master");
}

// ── TEST 5 (W14, MESH-02/03): 3-node gossip convergence over real QUIC ──
//
// Each node is a `GossipAgent` seeded with ONE distinct anchor endpoint. A
// background `listen` loop serves inbound rosters; `tick` dials every known
// peer, pushes the agent's full roster, and merges responses. After enough
// ticks, all three `PeerDirectory::snapshot_root` are IDENTICAL — full mesh
// discovery propagated purely by hand-rolled, full-roster anti-entropy gossip
// over the EXISTING `QuicTransport` (no DHT, no new deps). Real QUIC, no mocks.
//
// Note: the gossip layer is independent of the event-log sync layer above; it
// converges *peer identity/endpoints*, which is the MESH-02/03 surface.

#[cfg(feature = "insecure-tls")]
#[tokio::test]
async fn gossip_converges_3node() {
    let _lock = QUIC_PORT_LOCK.lock().await;

    // One trust anchor vouching for all 3 nodes (anchored allow-list model).
    let (anchor_seed, anchor_pk) = actor(1);

    // Three distinct node identities + stable loopback QUIC ports.
    let (a_seed, a_pk) = actor(2);
    let (b_seed, b_pk) = actor(3);
    let (c_seed, c_pk) = actor(4);
    let a_addr = free_port().await;
    let b_addr = free_port().await;
    let c_addr = free_port().await;

    // Anchored delegation chain (anchor -> node) scoped to Presence::Send, so
    // each agent's gossip frame satisfies the RequireBoth hybrid gate.
    let chain_for = |pk: [u8; 32]| -> Vec<Delegation> {
        vec![Delegation::sign(
            anchor_pk,
            pk,
            Scope::single(Resource::Presence, Action::Send),
            Effect::single(Resource::Presence, Action::Send),
            EXPIRY,
            [1u8; 8],
            &anchor_seed,
        )
        .expect("anchor delegation must sign")]
    };
    let mut roster = AnchorRoster::new();
    roster.enroll(&anchor_pk);

    // Build the three agents. Each starts EMPTY (MESH-02: discovery via gossip).
    let a = Arc::new(GossipAgent::new(
        a_pk,
        a_addr.clone(),
        a_seed,
        [10u8; 32],
        roster.clone(),
        chain_for(a_pk),
    ));
    let b = Arc::new(GossipAgent::new(
        b_pk,
        b_addr.clone(),
        b_seed,
        [11u8; 32],
        roster.clone(),
        chain_for(b_pk),
    ));
    let c = Arc::new(GossipAgent::new(
        c_pk,
        c_addr.clone(),
        c_seed,
        [12u8; 32],
        roster.clone(),
        chain_for(c_pk),
    ));

    // MESH-02 bootstrap: seed each agent with ONE distinct other anchor.
    a.add_self();
    a.seed_peer(b_pk, b_addr.clone());
    b.add_self();
    b.seed_peer(c_pk, c_addr.clone());
    c.add_self();
    c.seed_peer(a_pk, a_addr.clone());

    // Start the inbound listen loops (each serves its own roster, full duplex).
    tokio::spawn(a.spawn_listen());
    tokio::spawn(b.spawn_listen());
    tokio::spawn(c.spawn_listen());

    // Give the accept loops a moment to bind their stable ports.
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    // MESH-03: gossip until all three directories converge on the same root.
    let mut converged = false;
    for _round in 0..12u32 {
        // Drive ticks concurrently (each dials its known peers).
        let _ = tokio::join!(a.tick(), b.tick(), c.tick());
        let ra = a.dir.lock().unwrap().snapshot_root();
        let rb = b.dir.lock().unwrap().snapshot_root();
        let rc = c.dir.lock().unwrap().snapshot_root();
        if ra == rb && rb == rc && a.dir.lock().unwrap().len() == 3 {
            converged = true;
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }

    // Every node learned all three endpoints via gossip (no central registry).
    assert!(converged, "3-node gossip must converge to an identical roster");
    assert_eq!(a.dir.lock().unwrap().len(), 3);
    assert_eq!(b.dir.lock().unwrap().len(), 3);
    assert_eq!(c.dir.lock().unwrap().len(), 3);

    // All three snapshot roots identical => full mesh discovery.
    let ra = a.dir.lock().unwrap().snapshot_root();
    let rb = b.dir.lock().unwrap().snapshot_root();
    let rc = c.dir.lock().unwrap().snapshot_root();
    assert_eq!(ra, rb, "A and B agree on the peer set");
    assert_eq!(rb, rc, "B and C agree on the peer set");

    // Sanity: the converged set contains exactly the three self-endpoints.
    fn has(dir: &PeerDirectory, pk: [u8; 32], ep: &str) -> bool {
        dir.entry(&pk)
            .map(|e: &SignedEndpoint| e.endpoint == ep)
            .unwrap_or(false)
    }
    let da = a.dir.lock().unwrap();
    assert!(has(&da, a_pk, &a_addr));
    assert!(has(&da, b_pk, &b_addr));
    assert!(has(&da, c_pk, &c_addr));
}
