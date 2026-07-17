//! stdio transport — a carrier-neutral `Transport` over any `AsyncRead`/`AsyncWrite`
//! byte stream (in-memory duplex, OS pipe, or a spawned subprocess's stdin/stdout).
//!
//! This implements the **same** [`crate::Transport`] contract as `iroh_transport`
//! and `wss_transport`, so it is a drop-in carrier for `MeshNode<T: Transport>`.
//! It carries the **same** strict [`bebop_proto_cap::SignedFrame`] wire codec
//! (`wire_codec::encode_frame` / `decode_frame`) wrapped in the **same**
//! length-prefixed [`crate::framing`] envelope — there is **no** new lenient
//! parse path. **F11 (wire-format rejection) is inherited verbatim**: a malformed
//! frame that the strict codec rejects on the WSS/QUIC path is rejected here too,
//! because we call the very same `wire_codec::decode_frame` on `recv`.
//!
//! M6 (transport-swap leg): the wire / trust boundary stays zero-dep — only the
//! carrier is swapped. The signing/verify layer is untouched: `send` encodes the
//! already-signed frame; `recv` verifies through the [`bebop_proto_cap::HybridGate`]
//! exactly like `wss_transport`.
//!
//! CI GUARD: NO-COURIER-SCORING — the transport moves signed frames; it never
//! grades the mover.
//!
//! ── §7 acceptance (P09) ──────────────────────────────────────────────────────
//! * At least one new transport implements the `Transport` trait and passes the
//!   same integration suite (send → recv → verify at the far end + strict-codec
//!   reject). That is this module.
//! * Zero dependencies added to the signing path (M6 zero-dep boundary intact):
//!   this module adds NO crate; it reuses `tokio` (already present, incl.
//!   `tokio::io::duplex` for in-process tests) and `bebop2-core`/`bebop-proto-cap`.
//!
//! innovate: HTTP transport (the other §7 candidate) is **deliberately deferred**,
//! not omitted by accident. The blueprint's hard constraint is "zero new crates —
//! `Cargo.lock` MUST stay unchanged". A real HTTP/1.1 carrier needs BOTH a client
//! and a server runtime; the only HTTP-typed crate already present (`http`) is
//! type-only (no client/server IO). Hand-rolling an HTTP/1.1 request/response
//! duplex over `tokio::net::TcpStream` would be ~300+ fragile lines AND still
//! mismatch the trait's continuous `send`/`recv` stream model (HTTP is
//! request/response, not full-duplex framing). The carrier-swap PRINCIPLE this
//! anchor exists to prove is demonstrated completely by the stdio carrier: it is
//! a second, independent implementation of the identical `Transport` contract
//! that reuses the identical strict codec. When an HTTP client/server lib is
//! vendored without breaking the offline build, a `HttpTransport` can be added as
//! a mechanical third `impl Transport` reusing `wire_codec` + `framing` unchanged.
//! This is the documented upgrade trigger (no silent lenient-HTTP parser).

use std::sync::Arc;

use bebop_proto_cap::roster::AnchorRoster;
use bebop_proto_cap::scope::{Action, Resource};
use bebop_proto_cap::{BREACH_ALERT_BYTES, HybridGate, HybridPolicy, RevocationSet, SignedFrame};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::sync::Mutex;

use crate::error::{WireError, WireResult};
use crate::framing;
use crate::Transport;
// Reuse `wss_transport`'s node-scoped replay ledger (MESH-10) so a nonce consumed
// on one connection is rejected when replayed on another — same defense as WSS.
use crate::wss_transport::ReplayLedger;

/// A stdio endpoint descriptor.
///
/// stdio is a **symmetric** pipe: there is no client/server distinction in the
/// byte stream itself, so `connect` and `accept` are identical — both wrap the
/// same `read`/`write` pair. (The `connect`/`accept` split exists only to satisfy
/// the trait signature; for stdio they do the same thing, exactly as a subprocess
/// mesh would: the child reads its `stdin` and writes its `stdout`, the parent
/// does the inverse.)
///
/// The stream ends are held behind `Arc<Mutex<…>>` (not moved out) so the endpoint
/// remains a cheap, cloneable *descriptor* that `connect`/`accept` can take by
/// shared reference — mirroring how `MemEndpoint` clones an `Arc`-based `Link` and
/// how `WssEndpoint` clones a URL string.
#[derive(Clone)]
pub enum StdioEndpoint {
    /// A pre-wired carrier: `read` is the inbound stream, `write` is the outbound
    /// stream. Both must be `Unpin + Send`. Build one with [`StdioEndpoint::pipe`]
    /// (in-process) or by wrapping real OS pipe / subprocess streams.
    Stream {
        read: Arc<Mutex<Box<dyn AsyncRead + Unpin + Send>>>,
        write: Arc<Mutex<Box<dyn AsyncWrite + Unpin + Send>>>,
    },
}

impl StdioEndpoint {
    /// Build a connected, in-process full-duplex pair of endpoints.
    ///
    /// Returns `(client, server)`. What `client` writes, `server` reads; what
    /// `server` writes, `client` reads. No real OS fd / socket is touched, so
    /// this is the unit-test / subprocess-composition primitive (it uses
    /// `tokio::io::duplex`, already available in the `tokio` dependency).
    pub fn pipe() -> (StdioEndpoint, StdioEndpoint) {
        let (a, b) = tokio::io::duplex(64 * 1024);
        let (a_r, a_w) = tokio::io::split(a);
        let (b_r, b_w) = tokio::io::split(b);
        let client = StdioEndpoint::Stream {
            read: Arc::new(Mutex::new(Box::new(a_r))),
            write: Arc::new(Mutex::new(Box::new(a_w))),
        };
        let server = StdioEndpoint::Stream {
            read: Arc::new(Mutex::new(Box::new(b_r))),
            write: Arc::new(Mutex::new(Box::new(b_w))),
        };
        (client, server)
    }

    /// Wrap a single `AsyncRead + AsyncWrite` full-duplex stream (e.g. a real OS
    /// pipe obtained from `tokio::process`, or `tokio::io::duplex`) into a
    /// stdio endpoint. The same stream is used for both directions.
    pub fn from_duplex<S>(stream: S) -> StdioEndpoint
    where
        S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
    {
        let (r, w) = tokio::io::split(stream);
        StdioEndpoint::Stream {
            read: Arc::new(Mutex::new(Box::new(r))),
            write: Arc::new(Mutex::new(Box::new(w))),
        }
    }
}

// Manual Debug: never print stream internals (they are not `Debug`-friendly and
// may carry sensitive pipe state). Just identify the variant.
impl std::fmt::Debug for StdioEndpoint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StdioEndpoint::Stream { .. } => f.write_str("StdioEndpoint::Stream"),
        }
    }
}

/// An active stdio session: one peer's carrier stream + decode buffer + verify gate.
/// No score, no reputation — neutral plumbing.
pub struct StdioTransport {
    read: Arc<Mutex<Box<dyn AsyncRead + Unpin + Send>>>,
    write: Arc<Mutex<Box<dyn AsyncWrite + Unpin + Send>>>,
    /// Reassembly buffer for the length-prefixed framing.
    buf: Vec<u8>,
    /// Hybrid gate used to verify every received frame (classical live; PQ forced).
    gate: HybridGate,
    /// Enrolled trust-anchor roster consulted by the gate on every `recv`.
    roster: AnchorRoster,
    /// UCAN-style revocation set (MESH-11) consulted on every `recv`.
    revocations: RevocationSet,
    /// NODE-SCOPED replay ledger (MESH-10), shared across connections on a node.
    replay: ReplayLedger,
    /// Maximum accepted envelope size on the wire (DoS cap, MESH-10).
    max_frame_bytes: usize,
}

impl StdioTransport {
    fn from_arcs(
        read: Arc<Mutex<Box<dyn AsyncRead + Unpin + Send>>>,
        write: Arc<Mutex<Box<dyn AsyncWrite + Unpin + Send>>>,
    ) -> Self {
        StdioTransport {
            read,
            write,
            buf: Vec::new(),
            // PQ-IN-FORCE: RequireBoth on the live carrier (closes red-team H5).
            gate: HybridGate::new(HybridPolicy::RequireBoth),
            roster: AnchorRoster::new(),
            revocations: RevocationSet::new(),
            replay: ReplayLedger::new(65_536),
            max_frame_bytes: crate::framing::MAX_ENVELOPE_BYTES,
        }
    }

    /// Set the hybrid gate (defaults to `RequireBoth`).
    pub fn with_gate(self, gate: HybridGate) -> Self {
        StdioTransport { gate, ..self }
    }
    /// Attach a NODE-SCOPED replay ledger.
    pub fn with_replay_ledger(self, ledger: ReplayLedger) -> Self {
        StdioTransport { replay: ledger, ..self }
    }
    /// Set the enrolled trust-anchor roster used to verify delegation chains.
    pub fn with_roster(self, roster: AnchorRoster) -> Self {
        StdioTransport { roster, ..self }
    }
    /// Set the UCAN-style revocation set (MESH-11).
    pub fn with_revocations(self, revocations: RevocationSet) -> Self {
        StdioTransport {
            revocations,
            ..self
        }
    }
    /// Set the maximum accepted envelope size on the wire (DoS cap, MESH-10).
    pub fn with_max_frame_bytes(self, n: usize) -> Self {
        StdioTransport {
            max_frame_bytes: n,
            ..self
        }
    }
}

impl Transport for StdioTransport {
    type Endpoint = StdioEndpoint;

    async fn connect(endpoint: &Self::Endpoint) -> WireResult<Self> {
        match endpoint {
            StdioEndpoint::Stream { read, write } => {
                Ok(StdioTransport::from_arcs(Arc::clone(read), Arc::clone(write)))
            }
        }
    }

    async fn accept(endpoint: &Self::Endpoint) -> WireResult<Self> {
        // stdio is symmetric: `accept` wraps the same streams as `connect`.
        match endpoint {
            StdioEndpoint::Stream { read, write } => {
                Ok(StdioTransport::from_arcs(Arc::clone(read), Arc::clone(write)))
            }
        }
    }

    async fn send(&mut self, frame: SignedFrame) -> WireResult<()> {
        // G1 (2026-07-14): canonical binary codec (strict, fail-closed).
        let inner = crate::wire_codec::encode_frame(&frame)?;
        let envelope = crate::envelope::Envelope::new([0u8; 16], inner);
        let bytes = framing::encode(&envelope)?;
        {
            let mut w = self.write.lock().await;
            w.write_all(&bytes)
                .await
                .map_err(|e| WireError::Io(e.to_string()))?;
            w.flush().await.map_err(|e| WireError::Io(e.to_string()))?;
        }
        Ok(())
    }

    async fn recv(&mut self) -> WireResult<SignedFrame> {
        loop {
            // Try to decode a complete envelope from the buffer first.
            if let Some(env) = framing::decode(&mut self.buf)? {
                if env.payload.len() > self.max_frame_bytes {
                    return Err(WireError::PayloadTooLarge(env.payload.len()));
                }
                // G1: decode the SAME strict wire codec the other carriers use.
                // A malformed/version-skewed/corrupted frame is rejected HERE —
                // this is the F11 regression guard, reused not reimplemented.
                let frame: SignedFrame = crate::wire_codec::decode_frame(&env.payload)?;

                // NODE-SCOPED replay defense (MESH-10, B3-F2): record-before-verify
                // so a frame replayed on a different connection is rejected.
                let nonce = frame.capability.nonce;
                if !self.replay.observe(nonce) {
                    return Err(WireError::ReplayDetected(nonce));
                }

                // ── Воля АНУ breach: pure-P2P self-signed fail-safe ──────────────
                // A breach alarm is a legitimate self-signed frame (signed by the
                // node's OWN hybrid keys, not anchor-rooted). It MUST reach peers
                // directly; verify ONLY the real hybrid signature.
                if frame.capability.scope.grants == &[(Resource::BreachAlarm, Action::Broadcast)] {
                    if frame.verify_classical().is_err() || frame.verify_pq().is_err() {
                        return Err(WireError::HandshakeRejected(
                            "breach frame failed hybrid verify".into(),
                        ));
                    }
                    if frame.payload.len() != BREACH_ALERT_BYTES {
                        return Err(WireError::PayloadTooLarge(frame.payload.len()));
                    }
                    return Ok(frame);
                }

                // Verify the capability through the hybrid gate (anchor-rooted
                // delegation chain + real classical sig + real PQ sig + replay +
                // expiry). `now` is the REAL wall-clock tick.
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                self.gate.check(
                    &frame,
                    &self.roster,
                    &frame.delegation_chain,
                    &self.revocations,
                    now,
                )?;
                return Ok(frame);
            }
            // Need more bytes: read from the carrier stream.
            let mut chunk = [0u8; 8192];
            let n = {
                let mut r = self.read.lock().await;
                r.read(&mut chunk)
                    .await
                    .map_err(|e| WireError::Io(e.to_string()))?
            };
            if n == 0 {
                return Err(WireError::Carrier("peer closed connection".into()));
            }
            self.buf.extend_from_slice(&chunk[..n]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bebop_proto_cap::roster::{AnchorRoster, Delegation, Effect};
    use bebop_proto_cap::scope::{Action, Resource, Scope};
    use bebop_proto_cap::{Capability, HybridGate, HybridPolicy, SignedFrame};
    use tokio::task;

    /// (seed, pk) for a deterministic Ed25519 key.
    fn key(seed_byte: u8) -> ([u8; 32], [u8; 32]) {
        let seed = [seed_byte; 32];
        let (pk, _) = bebop2_core::sign::keygen(&seed);
        (seed, pk)
    }

    /// Build a frame signed by `leaf`, plus an anchor-rooted delegation chain
    /// (anchor -> leaf) carrying the same scope, and a roster enrolling anchor.
    /// The capability is HYBRID (real ML-DSA-65 `subject_key_pq`, signed under
    /// BOTH classical + PQ legs) so it satisfies the live `RequireBoth` gate.
    fn anchored_frame(
        anchor_seed: &[u8; 32],
        anchor_pk: &[u8; 32],
        leaf_seed: &[u8; 32],
        leaf_pk: &[u8; 32],
        resource: Resource,
        action: Action,
        nonce: [u8; 8],
        expiry: u64,
    ) -> (SignedFrame, AnchorRoster, Vec<Delegation>) {
        let (pq_pk, pq_sk) = bebop2_core::pq_dsa::keygen_derivable(leaf_seed);
        let cap = Capability::new_hybrid(
            *leaf_pk,
            pq_pk.bytes.clone(),
            resource,
            action,
            nonce,
            expiry,
        );
        let mut f = SignedFrame::new(cap, b"wire-payload".to_vec());
        f.sign_classical(leaf_seed).unwrap();
        f.sign_pq(
            &pq_sk.bytes.clone().try_into().unwrap(),
            &[0u8; 32],
        )
        .unwrap();
        let link = Delegation::sign(
            *anchor_pk,
            *leaf_pk,
            Scope::single(resource, action),
            Effect::single(resource, action),
            expiry,
            nonce,
            anchor_seed,
        )
        .unwrap();
        let mut roster = AnchorRoster::new();
        roster.enroll(anchor_pk);
        (f, roster, vec![link])
    }

    /// Drive an "accept loop" in a spawned task: accept one inbound connection on
    /// `server_ep`, run `body` with the connected transport (carrying `roster`).
    /// Mirrors the WSS test harness but stdio is symmetric, so `connect`/`accept`
    /// are interchangeable — we use `accept` to match the blueprint wording.
    async fn run_accept<F, Fut>(server_ep: StdioEndpoint, roster: AnchorRoster, body: F)
    where
        F: FnOnce(StdioTransport) -> Fut,
        Fut: core::future::Future<Output = ()>,
    {
        let t = StdioTransport::accept(&server_ep)
            .await
            .unwrap()
            .with_roster(roster);
        body(t).await;
    }

    #[tokio::test]
    async fn stdio_roundtrip_verifies_at_far_end() {
        // RED→GREEN: a frame signed by `leaf`, sent over the stdio carrier, must
        // be received AND pass the hybrid gate (RequireBoth) at the far end.
        let (anchor_seed, anchor_pk) = key(0xA1);
        let (leaf_seed, leaf_pk) = key(0xB2);
        let (frame, roster, chain) = anchored_frame(
            &anchor_seed,
            &anchor_pk,
            &leaf_seed,
            &leaf_pk,
            Resource::Route,
            Action::Send,
            [1u8; 8],
            9_999_999_999,
        );
        let mut frame = frame;
        frame.delegation_chain = chain;

        let (client_ep, server_ep) = StdioEndpoint::pipe();

        // Server task: accept, recv exactly one frame, verify it round-trips.
        let server = task::spawn(async move {
            run_accept(server_ep, roster, |mut t| async move {
                let got = t.recv().await.expect("server recv must succeed");
                assert_eq!(got.capability.nonce, [1u8; 8], "nonce preserved");
                assert_eq!(got.payload, b"wire-payload", "payload preserved");
                // The far end genuinely re-verifies the hybrid signature.
                assert!(got.verify_classical().is_ok(), "classical verify");
                assert!(got.verify_pq().is_ok(), "pq verify");
            })
            .await;
        });

        // Client: connect, send the frame.
        let mut client = StdioTransport::connect(&client_ep)
            .await
            .unwrap();
        client.send(frame).await.expect("client send must succeed");

        server.await.unwrap();
    }

    #[tokio::test]
    async fn stdio_bidirectional_ping_pong() {
        // Both ends send and receive — proves full-duplex over one stdio pipe.
        let (anchor_seed, anchor_pk) = key(0xC3);
        let (leaf_seed, leaf_pk) = key(0xD4);
        let (frame_a, roster_a, chain_a) = anchored_frame(
            &anchor_seed,
            &anchor_pk,
            &leaf_seed,
            &leaf_pk,
            Resource::Route,
            Action::Send,
            [2u8; 8],
            9_999_999_999,
        );
        let mut frame_a = frame_a;
        frame_a.delegation_chain = chain_a;
        let (frame_b, roster_b, chain_b) = anchored_frame(
            &anchor_seed,
            &anchor_pk,
            &leaf_seed,
            &leaf_pk,
            Resource::Route,
            Action::Send,
            [3u8; 8],
            9_999_999_999,
        );
        let mut frame_b = frame_b;
        frame_b.delegation_chain = chain_b;

        let (client_ep, server_ep) = StdioEndpoint::pipe();

        let server = task::spawn(async move {
            let mut t = StdioTransport::accept(&server_ep)
                .await
                .unwrap()
                .with_roster(roster_b);
            t.send(frame_b).await.unwrap();
            let got = t.recv().await.unwrap();
            assert_eq!(got.capability.nonce, [2u8; 8]);
        });

        let mut client = StdioTransport::connect(&client_ep)
            .await
            .unwrap()
            .with_roster(roster_a);
        client.send(frame_a).await.unwrap();
        let got = client.recv().await.unwrap();
        assert_eq!(got.capability.nonce, [3u8; 8]);

        server.await.unwrap();
    }

    #[tokio::test]
    async fn stdio_rejects_malformed_frame_strict_codec() {
        // F11 proof: the stdio carrier must reject a frame whose inner
        // `SignedFrame` bytes are corrupted, using the SAME strict codec
        // (`wire_codec::decode_frame`) — NO lenient parse path.
        let (anchor_seed, anchor_pk) = key(0xE5);
        let (leaf_seed, leaf_pk) = key(0xF6);
        let (frame, roster, _) = anchored_frame(
            &anchor_seed,
            &anchor_pk,
            &leaf_seed,
            &leaf_pk,
            Resource::Route,
            Action::Send,
            [4u8; 8],
            9_999_999_999,
        );

        let (client_ep, server_ep) = StdioEndpoint::pipe();

        // Server receives the corrupted frame and MUST reject it.
        let server = task::spawn(async move {
            let mut t = StdioTransport::accept(&server_ep)
                .await
                .unwrap()
                .with_roster(roster);
            let res = t.recv().await;
            assert!(
                matches!(res, Err(WireError::Encode(_))),
                "strict codec MUST reject corrupted frame, got: {res:?}"
            );
        });

        // Client: hand-encode a valid envelope, then CORRUPT the inner frame
        // payload (flip a byte in the signed-frame wire bytes) and resend.
        let mut inner = crate::wire_codec::encode_frame(&frame).unwrap();
        // Flip a byte deep inside the payload (past magic[8] + version[1] +
        // field-count[1]) so the envelope JSON still decodes but the strict
        // frame decode fails. (WIRE_MAGIC is private; the layout is fixed: 8B
        // magic + 1B version + 1B field-count.)
        let corrupt_at = 8 + 1 + 4;
        inner[corrupt_at] ^= 0xFF;
        let envelope = crate::envelope::Envelope::new([0u8; 16], inner);
        let bytes = framing::encode(&envelope).unwrap();

        let mut client = StdioTransport::connect(&client_ep)
            .await
            .unwrap();
        {
            let mut w = client.write.lock().await;
            w.write_all(&bytes).await.unwrap();
            w.flush().await.unwrap();
        }

        server.await.unwrap();
    }

    #[tokio::test]
    async fn stdio_rejects_replayed_frame() {
        // MESH-10 replay defense: a frame whose nonce was already seen on a
        // connection must be rejected when replayed on a DIFFERENT connection
        // (shared node-scoped ledger).
        let (anchor_seed, anchor_pk) = key(0x11);
        let (leaf_seed, leaf_pk) = key(0x22);
        let ledger = ReplayLedger::new(1024);

        let (frame, roster, chain) = anchored_frame(
            &anchor_seed,
            &anchor_pk,
            &leaf_seed,
            &leaf_pk,
            Resource::Route,
            Action::Send,
            [5u8; 8],
            9_999_999_999,
        );
        let mut frame = frame;
        frame.delegation_chain = chain;

        let (client_ep, server_ep) = StdioEndpoint::pipe();

        // First connection consumes the nonce.
        let mut first = StdioTransport::connect(&client_ep)
            .await
            .unwrap()
            .with_roster(roster.clone())
            .with_replay_ledger(ledger.clone());
        first.send(frame.clone()).await.unwrap();

        let mut server = StdioTransport::accept(&server_ep)
            .await
            .unwrap()
            .with_roster(roster)
            .with_replay_ledger(ledger);
        let got = server.recv().await.unwrap();
        assert_eq!(got.capability.nonce, [5u8; 8], "first delivery ok");

        // Second connection replays the SAME frame — must be rejected.
        let mut replay = StdioTransport::connect(&client_ep)
            .await
            .unwrap();
        replay.send(frame).await.unwrap();
        let res = server.recv().await;
        assert!(
            matches!(res, Err(WireError::ReplayDetected(_))),
            "replay MUST be rejected, got: {res:?}"
        );
    }
}
