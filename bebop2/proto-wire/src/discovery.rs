//! MESH-02 (roster discovery) + MESH-03 (gossip propagation).
//!
//! Hand-rolled, full-roster anti-entropy gossip over the EXISTING `QuicTransport`
//! (real QUIC, zero new dependencies). NOT a DHT: periodic full-roster exchange
//! between allow-listed peers, which is exactly the right primitive for an
//! anchored allow-list mesh. "Just use libp2p" was rejected: it cannot build
//! offline (native deps) and fights the anchored allow-list trust model.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use bebop2_core::pq_dsa::keygen_derivable;
use bebop_proto_cap::roster::{AnchorRoster, Delegation};
use bebop_proto_cap::{Action, Capability, Resource, RevocationSet, SignedFrame};
use serde::{Deserialize, Serialize};

use crate::iroh_transport::{QuicEndpoint, QuicTransport};
use crate::Transport;

/// Peer identity: Ed25519 public key (32 bytes). Directory key.
pub type PeerId = [u8; 32];

/// A discovered peer: stable identity + dialable `host:port`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignedEndpoint {
    pub peer: PeerId,
    pub endpoint: String,
}

/// FNV-1a (64-bit) fold helper.
fn fnv1a(bytes: &[u8], mut h: u64) -> u64 {
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

/// In-memory, content-addressed peer directory (BTreeMap => deterministic order).
pub struct PeerDirectory {
    peers: BTreeMap<PeerId, SignedEndpoint>,
    revoked: RevocationSet,
}

impl Default for PeerDirectory {
    fn default() -> Self {
        Self::new()
    }
}

impl PeerDirectory {
    pub fn new() -> Self {
        PeerDirectory {
            peers: BTreeMap::new(),
            revoked: RevocationSet::new(),
        }
    }

    /// Insert a peer endpoint; returns true if newly learned (id new & not revoked).
    pub fn insert(&mut self, ep: SignedEndpoint) -> bool {
        if self.revoked.is_revoked_key(&ep.peer) {
            return false;
        }
        self.peers.insert(ep.peer, ep).is_none()
    }

    /// Fold another directory in; returns newly-learned peer ids (id-sorted).
    pub fn merge(&mut self, other: &PeerDirectory) -> Vec<PeerId> {
        let mut learned = Vec::new();
        for (id, ep) in &other.peers {
            if !self.peers.contains_key(id) && !self.revoked.is_revoked_key(id) {
                self.peers.insert(*id, ep.clone());
                learned.push(*id);
            }
        }
        learned
    }

    /// Drop peers present in `revs` (and remember the revocation).
    pub fn evict_revoked(&mut self, revs: &RevocationSet) {
        self.revoked.merge(revs);
        let gone: Vec<PeerId> = self
            .peers
            .keys()
            .copied()
            .filter(|id| self.revoked.is_revoked_key(id))
            .collect();
        for id in gone {
            self.peers.remove(&id);
        }
    }

    /// Deterministic fingerprint over sorted `(id, endpoint)`. Two dirs with the
    /// same peer set yield the same root regardless of insertion order.
    pub fn snapshot_root(&self) -> String {
        let mut h = 0xcbf2_9ce4_8422_2325u64;
        for (id, ep) in &self.peers {
            h = fnv1a(id, h);
            h = fnv1a(ep.endpoint.as_bytes(), h);
        }
        format!("{h:016x}")
    }

    pub fn len(&self) -> usize {
        self.peers.len()
    }
    pub fn is_empty(&self) -> bool {
        self.peers.is_empty()
    }
    pub fn contains(&self, id: &PeerId) -> bool {
        self.peers.contains_key(id)
    }
    pub fn entry(&self, id: &PeerId) -> Option<&SignedEndpoint> {
        self.peers.get(id)
    }
    pub fn endpoints(&self) -> impl Iterator<Item = &SignedEndpoint> {
        self.peers.values()
    }

    /// Wire format via the crate's existing serde_json (no new dependency).
    /// `BTreeMap<[u8;32], _>` cannot be JSON-encoded (object keys must be
    /// strings, not arrays), so we serialize the (id, endpoint) pairs as a
    /// `Vec` — valid JSON and still deterministic (peers is a BTreeMap, so the
    /// pairs are emitted in id-sorted order).
    pub fn to_wire(&self) -> Vec<u8> {
        let pairs: Vec<(&PeerId, &SignedEndpoint)> = self.peers.iter().collect();
        serde_json::to_vec(&pairs).unwrap_or_default()
    }
    pub fn from_wire(b: &[u8]) -> Self {
        let pairs: Vec<(PeerId, SignedEndpoint)> = serde_json::from_slice(b).unwrap_or_default();
        let mut peers = BTreeMap::new();
        for (id, ep) in pairs {
            peers.insert(id, ep);
        }
        PeerDirectory {
            peers,
            revoked: RevocationSet::new(),
        }
    }
}

/// Periodic full-roster gossip agent. Holds a stable `listen_addr` + shared
/// directory. A background [`listen_loop`] serves inbound rosters; [`tick`]
/// dials known peers, exchanges rosters, merges responses. MESH-02 = first
/// roster fetch from seeded peers; MESH-03 = re-gossip learned peers.
pub struct GossipAgent {
    pub id: PeerId,
    pub dir: Arc<Mutex<PeerDirectory>>,
    anchor_roster: AnchorRoster,
    chain: Vec<Delegation>,
    revocations: Arc<Mutex<RevocationSet>>,
    listen_addr: String,
    seed: [u8; 32],
    pq_seed: [u8; 32],
    nonce: Arc<AtomicU64>,
}

impl GossipAgent {
    pub fn new(
        id: PeerId,
        listen_addr: String,
        seed: [u8; 32],
        pq_seed: [u8; 32],
        anchor_roster: AnchorRoster,
        chain: Vec<Delegation>,
    ) -> Self {
        GossipAgent {
            id,
            dir: Arc::new(Mutex::new(PeerDirectory::new())),
            anchor_roster,
            chain,
            revocations: Arc::new(Mutex::new(RevocationSet::new())),
            listen_addr,
            seed,
            pq_seed,
            nonce: Arc::new(AtomicU64::new(1)),
        }
    }

    /// Register our own endpoint so gossip advertises it.
    pub fn add_self(&self) {
        self.dir.lock().unwrap().insert(SignedEndpoint {
            peer: self.id,
            endpoint: self.listen_addr.clone(),
        });
    }

    /// Seed the directory with one known peer endpoint (MESH-02 bootstrap).
    pub fn seed_peer(&self, peer: PeerId, endpoint: String) {
        self.dir.lock().unwrap().insert(SignedEndpoint { peer, endpoint });
    }

    /// Hybrid-signed frame carrying a roster payload (fresh nonce each call).
    /// Delegates to the free [`build_roster_frame`] so the spawned
    /// `listen_loop` future does NOT capture `&self`/a `MutexGuard` (keeps it `Send`).
    fn wrap_roster(&self, payload: Vec<u8>) -> SignedFrame {
        build_roster_frame(self.id, self.pq_seed, self.seed, &self.chain, &self.nonce, payload)
    }

    /// Spawn the background [`listen_loop`] on the tokio runtime. Clones the
    /// agent's own (private) `Send` fields into the `Send` future so callers
    /// don't reach into the struct. Returns the `JoinHandle`.
    pub fn spawn_listen(&self) -> tokio::task::JoinHandle<()> {
        tokio::spawn(listen_loop(
            self.dir.clone(),
            self.revocations.clone(),
            self.anchor_roster.clone(),
            self.chain.clone(),
            self.seed,
            self.pq_seed,
            self.id,
            self.listen_addr.clone(),
            self.nonce.clone(),
        ))
    }

    /// One gossip round: dial every known peer (except self), push our roster,
    /// merge their response. Returns peer ids learned this round.
    /// Not spawned -> `&self` borrow is fine (no `Send` bound).
    pub async fn tick(&self) -> Vec<PeerId> {
        let targets: Vec<SignedEndpoint> = self
            .dir
            .lock()
            .unwrap()
            .endpoints()
            .filter(|e| e.peer != self.id)
            .cloned()
            .collect();
        let mut learned = Vec::new();
        for t in targets {
            let payload = self.dir.lock().unwrap().to_wire();
            let frame = self.wrap_roster(payload);
            match QuicTransport::connect(&QuicEndpoint::Dial(t.endpoint.clone())).await {
                Ok(mut tr) => {
                    let mut tr = tr
                        .with_roster(self.anchor_roster.clone())
                        .with_revocations(self.revocations.lock().unwrap().clone());
                    match tr.send(frame).await {
                        Ok(_) => match tr.recv().await {
                            Ok(resp) => {
                                learned.extend(self.dir.lock().unwrap().merge(&PeerDirectory::from_wire(&resp.payload)));
                            }
                            Err(_) => {}
                        },
                        Err(e) => eprintln!("gossip tick send err to {}: {:?}", t.endpoint, e),
                    }
                }
                Err(e) => eprintln!("gossip tick connect err to {}: {:?}", t.endpoint, e),
            }
        }
        learned
    }
}

/// Builds a hybrid-signed roster frame. Free fn (no `&self`) so the spawned
/// `listen_loop` future holds ONLY `Send` params -> `tokio::spawn` is happy.
fn build_roster_frame(
    id: PeerId,
    pq_seed: [u8; 32],
    seed: [u8; 32],
    chain: &[Delegation],
    nonce: &Arc<AtomicU64>,
    payload: Vec<u8>,
) -> SignedFrame {
    let (pq_pk, pq_sk) = keygen_derivable(&pq_seed);
    let n = nonce.fetch_add(1, Ordering::SeqCst);
    let cap = Capability::new_hybrid(
        id,
        pq_pk.bytes.clone(),
        Resource::Presence,
        Action::Send,
        n.to_le_bytes(),
        9_999_999_999,
    );
    let mut f = SignedFrame::new(cap, payload);
    f.sign_classical(&seed).expect("classical sign");
    f.sign_pq(
        &pq_sk.bytes.clone().try_into().expect("pq sk width"),
        &[0u8; 32],
    )
    .expect("pq sign");
    f.delegation_chain = chain.to_vec();
    f
}

/// Background accept loop: accept one inbound QUIC connection, merge the
/// peer's roster, reply with ours. Rebinds the same stable port each accept.
///
/// Takes ONLY `Send` params (cloned `Arc`s + `Clone` fields) so the future is
/// `Send` and can be `tokio::spawn`-ed. Does NOT capture `&self` / a
/// `MutexGuard` across an `.await` (that made a prior `listen(self: Arc<Self>)`
/// future `!Send`).
async fn listen_loop(
    dir: Arc<Mutex<PeerDirectory>>,
    revocations: Arc<Mutex<RevocationSet>>,
    anchor_roster: AnchorRoster,
    chain: Vec<Delegation>,
    seed: [u8; 32],
    pq_seed: [u8; 32],
    id: PeerId,
    listen_addr: String,
    nonce: Arc<AtomicU64>,
) {
    // Bind the stable listening socket ONCE. Re-binding inside the loop would
    // drop the socket between inbound dials and cause connection refusals.
    let ep = QuicEndpoint::Bind(listen_addr);
    loop {
        match QuicTransport::accept(&ep).await {
            Ok(tr) => {
                // Handle each inbound connection in its OWN task. This keeps the
                // accept loop responsive AND — critically — keeps the connection's
                // QUIC `Endpoint` (owned by the server `QuicTransport`) alive
                // until the reply flushes. Dropping it inline (as a prior version
                // did) aborts the connection before the client reads its reply,
                // surfacing as `Carrier("connection lost")` on the client recv.
                // This mirrors `quic_roundtrip`, which holds the conn open until
                // the client reads.
                let dir = dir.clone();
                let revs = revocations.clone();
                let roster = anchor_roster.clone();
                let chain = chain.clone();
                let nonce = nonce.clone();
                tokio::spawn(async move {
                    handle_conn(tr, dir, revs, roster, chain, seed, pq_seed, id, nonce).await;
                });
            }
            Err(_) => tokio::time::sleep(Duration::from_millis(50)).await,
        }
    }
}

/// Serve ONE inbound gossip connection: merge the peer's roster, reply with
/// ours. Holds the `QuicTransport` (and thus its QUIC `Endpoint`) alive for a
/// grace period after the reply so the client can read it before the conn is
/// closed.
async fn handle_conn(
    mut tr: QuicTransport,
    dir: Arc<Mutex<PeerDirectory>>,
    revocations: Arc<Mutex<RevocationSet>>,
    anchor_roster: AnchorRoster,
    chain: Vec<Delegation>,
    seed: [u8; 32],
    pq_seed: [u8; 32],
    id: PeerId,
    nonce: Arc<AtomicU64>,
) {
    tr = tr
        .with_roster(anchor_roster)
        .with_revocations(revocations.lock().unwrap().clone());
    match tr.recv().await {
        Ok(frame) => {
            dir.lock().unwrap().merge(&PeerDirectory::from_wire(&frame.payload));
            let payload = dir.lock().unwrap().to_wire();
            let f = build_roster_frame(id, pq_seed, seed, &chain, &nonce, payload);
            match tr.send(f).await {
                Ok(_) => {
                    // Hold the connection open with an explicit grace sleep
                    // (NOT by reading the client's stream). The client's send
                    // stream is already STREAM_FIN-ished right after its frame,
                    // so `tr.recv()` would return immediately and drop the
                    // endpoint -> the reply is RST before the client reads it
                    // ("connection lost"). A bounded sleep keeps the QUIC
                    // `Endpoint` (owned by `tr`) alive long enough for the reply
                    // to flush and the client to consume it.
                    tokio::time::sleep(Duration::from_millis(500)).await;
                }
                Err(e) => eprintln!("gossip handle_conn send err: {:?}", e),
            }
        }
        Err(e) => eprintln!("gossip handle_conn recv err: {:?}", e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bebop_proto_cap::roster::{AnchorRoster, Delegation, Effect};
    use bebop_proto_cap::scope::{Action, Resource, Scope};
    use bebop_proto_cap::{Capability, SignedFrame};
    use crate::Transport;
    use std::sync::Mutex;

    fn ep(i: u8) -> SignedEndpoint {
        SignedEndpoint {
            peer: [i; 32],
            endpoint: format!("127.0.0.1:{}", 7000u16 + i as u16),
        }
    }

    #[test]
    fn peer_directory_merge_dedup() {
        let mut a = PeerDirectory::new();
        let mut b = PeerDirectory::new();
        a.insert(ep(1));
        a.insert(ep(2));
        b.insert(ep(2));
        b.insert(ep(3));
        let learned = a.merge(&b);
        assert_eq!(learned, vec![[3u8; 32]]);
        assert_eq!(a.len(), 3);
        // Idempotent: merging again learns nothing.
        assert!(a.merge(&b).is_empty());
    }

    #[test]
    fn snapshot_root_is_deterministic() {
        let mut a = PeerDirectory::new();
        let mut b = PeerDirectory::new();
        a.insert(ep(3));
        a.insert(ep(1));
        b.insert(ep(1));
        b.insert(ep(3));
        assert_eq!(a.snapshot_root(), b.snapshot_root());
    }

    #[test]
    fn wire_roundtrip_preserves_peers() {
        let mut a = PeerDirectory::new();
        a.insert(ep(1));
        a.insert(ep(2));
        let w = a.to_wire();
        let b = PeerDirectory::from_wire(&w);
        assert_eq!(b.len(), 2, "wire roundtrip lost peers: {:?}", String::from_utf8_lossy(&w));
        assert_eq!(a.snapshot_root(), b.snapshot_root());
    }

    #[test]
    fn revocation_evicts() {
        let mut d = PeerDirectory::new();
        d.insert(ep(1));
        d.insert(ep(2));
        let mut revs = RevocationSet::new();
        revs.revoke_key(ep(2).peer);
        d.evict_revoked(&revs);
        assert!(!d.contains(&[2u8; 32]), "revoked peer dropped");
        assert_eq!(d.len(), 1);
    }
}
