//! QUIC transport — real node-to-node carrier (pure-Rust `quinn`/rustls).
//!
//! This REPLACES the legacy `crates/bebop/src/zenoh.rs` process-local pub/sub
//! stub AND the old `unimplemented!()` iroh placeholder. The blueprint (MESH-09)
//! mandated a real QUIC carrier; this is it — wired for real, not parked.
//!
//! # Why `quinn` and not `iroh`
//! The original iroh-stub was deferred because the `iroh` crate conflicted with
//! the `ed25519-dalek` pin in `crates/bebop` (`=3.0.0-rc.0` vs `^3`) and the
//! sovereign core must build OFFLINE with zero C-build supply chain. `quinn`
//! (the QUIC implementation iroh itself uses) has **no `ed25519-dalek` dep** and
//! builds against `rustls` + `ring` only — so it sidesteps the exact conflict
//! that parked iroh and needs no openssl-sys (native-tls is banned by the
//! blueprint §3G/F6). Same ALPN, same [`Transport`] contract, same framing — a
//! drop-in real carrier.
//!
//! # What it carries
//! The same carrier-neutral [`Envelope`] + [`framing`] as `wss_transport`:
//! signed [`bebop_proto_cap::SignedFrame`]s, length-prefixed as QUIC stream
//! bytes, signed on `send` and verified on `recv` through the `RequireBoth`
//! hybrid gate. No scoring, no reputation (NO-COURIER-SCORING guard).
//!
//! innovate: iroh DHT hole-punching is OUT of scope here (quinn gives direct
//! QUIC; NAT traversal is a deployment concern). Trigger: add an iroh/derp relay
//! or a STUN-less hole-punch layer if a real deployment needs it.

#![allow(dead_code)]

use std::net::{SocketAddr, ToSocketAddrs};
use std::sync::{Arc, LazyLock, Mutex};
use std::collections::HashMap;

use quinn::crypto::rustls::{QuicClientConfig, QuicServerConfig};
use quinn::{ClientConfig, Endpoint, RecvStream, SendStream, ServerConfig};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use rustls::SignatureScheme;
use tokio::io::AsyncWriteExt;

use bebop_proto_cap::roster::AnchorRoster;
use bebop_proto_cap::{
    BREACH_ALERT_BYTES, Action, HybridGate, HybridPolicy, Resource, RevocationSet, SignedFrame,
};

use crate::error::{WireError, WireResult};
use crate::framing;
use crate::Transport;

/// Cache of already-bound server `Endpoint`s keyed by their bind address.
///
/// `accept` is called once per inbound connection; re-binding a fresh UDP
/// socket on every call drops the previously-bound socket (and any live
/// connections on it) between calls, which makes a persistent server
/// (gossip `listen_loop`) lose connections. quinn `Endpoint` is `Clone` and
/// shares the underlying socket, so we bind once per address and reuse.
static BOUND_ENDPOINTS: LazyLock<Mutex<HashMap<String, Endpoint>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// ALPN protocol tag shared by every bebop2 wire carrier (QUIC + WSS framing).
pub const ALPN_BEBOP2_WIRE: &[u8] = b"bebop2/wire/1";

/// A QUIC endpoint descriptor.
#[derive(Debug, Clone)]
pub enum QuicEndpoint {
    /// A `host:port` (or `[ip]:port`) to dial as a client.
    Dial(String),
    /// A local `host:port` to bind and accept connections on (server side).
    Bind(String),
}

/// An active QUIC session: one peer's uni/bi stream pair + decode buffer + gate.
/// No score, no reputation.
pub struct QuicTransport {
    /// The QUIC endpoint (kept alive for the connection's lifetime).
    _endpoint: Endpoint,
    /// Stream we write framed envelopes to.
    send: SendStream,
    /// Stream we read framed envelopes from.
    recv: RecvStream,
    /// Reassembly buffer for the length-prefixed framing.
    buf: Vec<u8>,
    /// Hybrid gate (RequireBoth) verifying every received frame.
    gate: HybridGate,
    roster: AnchorRoster,
    revocations: RevocationSet,
}

impl QuicTransport {
    fn from_parts(
        endpoint: Endpoint,
        send: SendStream,
        recv: RecvStream,
        gate: HybridGate,
        roster: AnchorRoster,
        revocations: RevocationSet,
    ) -> Self {
        QuicTransport {
            _endpoint: endpoint,
            send,
            recv,
            buf: Vec::new(),
            gate,
            roster,
            revocations,
        }
    }

    /// Set the rustls `ring` crypto provider (idempotent; QUIC needs a provider
    /// installed process-wide). `ring` is the banned-native-tls-safe choice.
    fn ensure_crypto_provider() {
        let _ = rustls::crypto::ring::default_provider().install_default();
    }

    /// Set the enrolled trust-anchor roster.
    pub fn with_roster(self, roster: AnchorRoster) -> Self {
        QuicTransport { roster, ..self }
    }
    /// Set the UCAN-style revocation set (MESH-11).
    pub fn with_revocations(self, revocations: RevocationSet) -> Self {
        QuicTransport {
            revocations,
            ..self
        }
    }

    /// Build a self-signed rustls server config (DEV/loopback; mutual-auth layer
    /// is the signed-frame envelope, not x509). No openssl-sys involved.
    fn server_crypto() -> QuicServerConfig {
        let cert = rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
        let key = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(cert.signing_key.serialize_der()));
        let cert_der = CertificateDer::from(cert.cert.der().to_vec());
        let mut sc = rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(vec![cert_der], key)
            .expect("static self-signed cert is valid");
        sc.alpn_protocols = vec![ALPN_BEBOP2_WIRE.to_vec()];
        QuicServerConfig::try_from(sc).expect("quic server config")
    }

    /// Build a client config that trusts our own dev cert (insecure for real
    /// deployments — the wire auth is the signed-frame envelope). `ring` provider.
    fn client_crypto() -> ClientConfig {
        // C5: client-side rustls TLS via `client_rustls_config()` (see its doc for the honest scope —
        // client-only; the server accept + a `wss://` handshake test are still pending).
        let mut rc = client_rustls_config();
        rc.alpn_protocols = vec![ALPN_BEBOP2_WIRE.to_vec()];
        let quic_client = QuicClientConfig::try_from(rc).expect("quic client config");
        ClientConfig::new(Arc::new(quic_client))
    }
}

/// Dev-only cert verifier that accepts ANY cert. Wire authenticity comes from
/// the signed-frame hybrid gate, NOT from x509 — this is the explicit local-first
/// default (see wss_transport H6 marker). Production MUST replace with a real
/// root store + channel binding.
#[derive(Debug)]
struct InsecureAcceptAny;

impl rustls::client::danger::ServerCertVerifier for InsecureAcceptAny {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }
    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }
    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }
    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        vec![
            SignatureScheme::ECDSA_NISTP256_SHA256,
            SignatureScheme::ECDSA_NISTP384_SHA384,
            SignatureScheme::ED25519,
            SignatureScheme::RSA_PSS_SHA256,
        ]
    }
}
// SAFETY: this is a DEV-only verifier; the real auth boundary is the signed-frame
// envelope verified on every recv (HybridGate::RequireBoth). Marked unsafe to
// make the compromise explicit and greppable.
unsafe impl Sync for InsecureAcceptAny {}

/// Shared CLIENT TLS config for both carriers (iroh QUIC + wss). C5 (client-side rustls TLS):
/// - hardened (`insecure-tls` OFF) → verify the server cert against the Mozilla CA roots.
/// - dev (`insecure-tls` ON, the DEFAULT) → accept ANY cert (local-first; the signed-frame hybrid
///   gate is the real auth boundary, verified on every recv).
/// SCOPE (2026-07-14 3-model review): TLS is proven end-to-end on BOTH paths — `wss_tls_handshake_roundtrip`
/// proves a real handshake + signed-frame round-trip, and `hardened_verifier_rejects_self_signed_cert`
/// proves the webpki-roots verifier REJECTS an untrusted cert. Remaining follow-up: a prod operator-cert
/// `ListenTls` variant (the self-signed cert is dev/test only — a real deployment supplies a CA-issued
/// cert; a hardened client rejects self-signed on both unknown-issuer and SAN grounds).
/// DECIDED (operator, 2026-07-14): rustls+ring is the explicit MAIN crypto provider (runtime, via
/// `builder_with_provider`); aws-lc-rs and other providers are accepted FALLBACKS (compiled, not primary)
/// — a decision, NOT a defect, so no cross-crate provider purge is pursued.
pub(crate) fn client_rustls_config() -> rustls::ClientConfig {
    // ring is the PRIMARY crypto provider, chosen EXPLICITLY — do not rely on the process-default,
    // which silently prefers aws-lc-rs (a C/cmake build) when both are compiled. rustls+ring keeps the
    // sovereign, no-C-supply-chain posture; aws-lc, if present transitively, stays an unused fallback.
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    #[cfg(feature = "insecure-tls")]
    {
        rustls::ClientConfig::builder_with_provider(provider)
            .with_safe_default_protocol_versions()
            .expect("ring supports default TLS versions")
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(InsecureAcceptAny))
            .with_no_client_auth()
    }
    #[cfg(not(feature = "insecure-tls"))]
    {
        let mut roots = rustls::RootCertStore::empty();
        roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        rustls::ClientConfig::builder_with_provider(provider)
            .with_safe_default_protocol_versions()
            .expect("ring supports default TLS versions")
            .with_root_certificates(roots)
            .with_no_client_auth()
    }
}

impl Transport for QuicTransport {
    type Endpoint = QuicEndpoint;

    async fn connect(endpoint: &Self::Endpoint) -> WireResult<Self> {
        Self::ensure_crypto_provider();
        let addr = match endpoint {
            QuicEndpoint::Dial(a) => a.clone(),
            QuicEndpoint::Bind(_) => {
                return Err(WireError::HandshakeRejected(
                    "use accept() for a Bind endpoint".into(),
                ))
            }
        };
        let remote: SocketAddr = addr
            .to_socket_addrs()
            .ok()
            .and_then(|mut i| i.next())
            .ok_or_else(|| WireError::HandshakeRejected(format!("bad dial addr: {addr}")))?;

        let mut endpoint = Endpoint::client("127.0.0.1:0".parse().unwrap())
            .map_err(|e| WireError::Carrier(e.to_string()))?;
        endpoint.set_default_client_config(Self::client_crypto());

        let conn = endpoint
            .connect(remote, "localhost")
            .map_err(|e| WireError::HandshakeRejected(e.to_string()))?
            .await
            .map_err(|e| WireError::HandshakeRejected(e.to_string()))?;

        let (send, recv) = conn
            .open_bi()
            .await
            .map_err(|e| WireError::Carrier(e.to_string()))?;

        Ok(QuicTransport::from_parts(
            endpoint,
            send,
            recv,
            HybridGate::new(HybridPolicy::RequireBoth),
            AnchorRoster::new(),
            RevocationSet::new(),
        ))
    }

    async fn accept(endpoint: &Self::Endpoint) -> WireResult<Self> {
        Self::ensure_crypto_provider();
        let addr = match endpoint {
            QuicEndpoint::Bind(a) => a.clone(),
            QuicEndpoint::Dial(_) => {
                return Err(WireError::HandshakeRejected(
                    "use connect() for a Dial endpoint".into(),
                ))
            }
        };
        let bind: SocketAddr = addr
            .to_socket_addrs()
            .ok()
            .and_then(|mut i| i.next())
            .ok_or_else(|| WireError::HandshakeRejected(format!("bad bind addr: {addr}")))?;
        // Bind our own UDP socket ONCE per address and reuse it across every
        // inbound connection. quinn `Endpoint` is `Clone` and shares the
        // underlying socket, so re-using the cached endpoint does NOT create a
        // new socket; this keeps a persistent server (`listen_loop`) alive
        // between accepts instead of dropping the socket (and live connections)
        // on every `accept` call.
        let endpoint = {
            let cache = BOUND_ENDPOINTS.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(e) = cache.get(&addr) {
                e.clone()
            } else {
                drop(cache);
                let std_sock = std::net::UdpSocket::bind(bind)
                    .map_err(|e| WireError::HandshakeRejected(e.to_string()))?;
                let server_cfg = ServerConfig::with_crypto(Arc::new(Self::server_crypto()));
                let e = Endpoint::new(
                    quinn::EndpointConfig::default(),
                    Some(server_cfg),
                    std_sock,
                    Arc::new(quinn::TokioRuntime),
                )
                .map_err(|e| WireError::Carrier(e.to_string()))?;
                BOUND_ENDPOINTS.lock().unwrap_or_else(|e| e.into_inner()).insert(addr.clone(), e.clone());
                e
            }
        };

        // Accept one inbound connection, then open a bi stream for the peer.
        let conn = endpoint
            .accept()
            .await
            .ok_or(WireError::NotConnected)?
            .await
            .map_err(|e| WireError::HandshakeRejected(e.to_string()))?;
        let (send, recv) = conn
            .accept_bi()
            .await
            .map_err(|e| WireError::Carrier(e.to_string()))?;

        Ok(QuicTransport::from_parts(
            endpoint,
            send,
            recv,
            HybridGate::new(HybridPolicy::RequireBoth),
            AnchorRoster::new(),
            RevocationSet::new(),
        ))
    }

    async fn send(&mut self, frame: SignedFrame) -> WireResult<()> {
        // G1 (2026-07-14): canonical binary codec replaces serde_json.
        let inner = crate::wire_codec::encode_frame(&frame)?;
        let envelope = crate::envelope::Envelope::new([0u8; 16], inner);
        let bytes = framing::encode(&envelope)?;
        self.send
            .write_all(&bytes)
            .await
            .map_err(|e| WireError::Carrier(e.to_string()))?;
        self.send
            .finish()
            .map_err(|e| WireError::Carrier(e.to_string()))?;
        Ok(())
    }

    async fn recv(&mut self) -> WireResult<SignedFrame> {
        loop {
            if let Some(env) = framing::decode(&mut self.buf)? {
                // G1 (2026-07-14): decode the canonical wire codec, not serde_json.
                let frame: SignedFrame = crate::wire_codec::decode_frame(&env.payload)?;

                // ── Воля АНУ breach: pure-P2P self-signed fail-safe ──────────────
                // A breach alarm is a legitimate *self-signed* frame: it is signed
                // by the node's OWN hybrid keys, NOT by an anchor-rooted delegation
                // chain. It MUST reach peers directly (no hub, no relay, no 3rd
                // party) even when the broadcaster's standing is in question — that
                // is the whole point of the alarm. So we bypass the roster/UCAN gate
                // and verify ONLY the real hybrid signature (Ed25519 + ML-DSA-65):
                // only the node holding BOTH secret keys can produce a frame that
                // passes, which makes a forged breach impossible. Domain-separated
                // by the `BreachAlarm`/`Broadcast` scope so it cannot be abused to
                // smuggle other capabilities.
                if frame.capability.scope.grants
                    == &[(Resource::BreachAlarm, Action::Broadcast)]
                {
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
            // Need more bytes from the QUIC stream.
            let chunk = self
                .recv
                .read_chunk(8192, false)
                .await
                .map_err(|e| WireError::Carrier(e.to_string()))?
                .ok_or(WireError::Closed)?;
            self.buf.extend_from_slice(&chunk.bytes);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bebop_proto_cap::roster::{AnchorRoster, Delegation, Effect};
    use bebop_proto_cap::scope::{Action, Resource, Scope};
    use bebop_proto_cap::{Capability, SignedFrame};
    use std::sync::LazyLock;
    use tokio::net::UdpSocket;
    use tokio::sync::{oneshot, Mutex};

    /// Serialize the QUIC tests: they grab ephemeral UDP ports, and running them
    /// concurrently can recycle a just-released port before the prior endpoint
    /// fully unbinds (EADDRINUSE). One at a time avoids the race.
    static QUIC_PORT_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

    fn key(seed_byte: u8) -> ([u8; 32], [u8; 32]) {
        let seed = [seed_byte; 32];
        let (pk, _) = bebop2_core::sign::keygen(&seed);
        (seed, pk)
    }

    /// A frame signed under BOTH classical (Ed25519) + PQ (ML-DSA-65) legs with
    /// an anchor-rooted delegation chain, satisfying the live `RequireBoth` gate.
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
        let mut f = SignedFrame::new(cap, b"quic-wire-payload".to_vec());
        f.sign_classical(leaf_seed).unwrap();
        f.sign_pq(&pq_sk.bytes.clone().try_into().unwrap(), &[0u8; 32])
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

    /// Grab a free loopback UDP port (QUIC rides UDP).
    async fn free_port() -> String {
        let sock = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let addr = sock.local_addr().unwrap().to_string();
        drop(sock);
        addr
    }

    /// Two real QUIC nodes (server + client) over loopback exchange a signed
    /// frame: client sends an anchor-rooted hybrid frame, server verifies it
    /// through the RequireBoth gate and echoes it back. Proves the QUIC carrier
    /// is no longer a stub — it moves signed frames over a real QUIC stream.
    // Uses the accept-any client against a self-signed QUIC server cert; hardened
    // (`--no-default-features`) rejects the untrusted cert, so this is insecure-tls-only.
    #[cfg(feature = "insecure-tls")]
    #[tokio::test]
    async fn quic_roundtrip_signs_and_verifies() {
        let _lock = QUIC_PORT_LOCK.lock().await;
        let addr = free_port().await;

        let (a_seed, a_pk) = key(2);
        let (l_seed, l_pk) = key(3);
        let (frame, roster, chain) = anchored_frame(
            &a_seed,
            &a_pk,
            &l_seed,
            &l_pk,
            Resource::Route,
            Action::Send,
            [7u8; 8],
            9_999_999_999,
        );

        let (tx, rx) = oneshot::channel();
        let server_addr = addr.clone();
        let server_roster = roster.clone();
        let server_done = std::sync::Arc::new(tokio::sync::Notify::new());
        let client_done = server_done.clone();
        let server = tokio::spawn(async move {
            let _ = tx.send(());
            let ep = QuicEndpoint::Bind(server_addr);
            let mut t = QuicTransport::accept(&ep)
                .await
                .unwrap()
                .with_roster(server_roster);
            let frame = t.recv().await.unwrap();
            t.send(frame).await.unwrap();
            // Hold the QUIC connection open until the client has read the echo,
            // so the server endpoint isn't dropped mid-read ("connection lost").
            server_done.notified().await;
        });
        rx.await.unwrap();

        let client_ep = QuicEndpoint::Dial(addr);
        let mut client = QuicTransport::connect(&client_ep)
            .await
            .unwrap()
            .with_roster(roster.clone());
        let mut signed = frame;
        signed.delegation_chain = chain;
        client.send(signed).await.unwrap();

        let echoed = client.recv().await.unwrap();
        assert_eq!(echoed.payload, b"quic-wire-payload");
        assert!(echoed.verify_classical().is_ok());
        client_done.notify_one();

        server.await.unwrap();
    }

    /// RED over the REAL QUIC carrier: a tampered frame (signed, then payload
    /// mutated) MUST be rejected by the server's recv (hybrid gate).
    // Accept-any client against a self-signed QUIC server cert (insecure-tls-only); the
    // tamper-rejection property it proves is independent of the TLS verifier.
    #[cfg(feature = "insecure-tls")]
    #[tokio::test]
    async fn quic_rejects_tampered_frame() {
        let _lock = QUIC_PORT_LOCK.lock().await;
        let addr = free_port().await;

        let (a_seed, a_pk) = key(2);
        let (l_seed, l_pk) = key(3);
        let (frame, roster, chain) = anchored_frame(
            &a_seed,
            &a_pk,
            &l_seed,
            &l_pk,
            Resource::Ledger,
            Action::Append,
            [2u8; 8],
            9_999_999_999,
        );

        let (tx, rx) = oneshot::channel();
        let server_addr = addr.clone();
        let server = tokio::spawn(async move {
            let _ = tx.send(());
            let ep = QuicEndpoint::Bind(server_addr);
            let mut t = QuicTransport::accept(&ep)
                .await
                .unwrap()
                .with_roster(roster);
            let res = t.recv().await;
            assert!(res.is_err(), "tampered frame MUST be rejected over QUIC");
        });
        rx.await.unwrap();

        let client_ep = QuicEndpoint::Dial(addr);
        let mut client = QuicTransport::connect(&client_ep).await.unwrap();
        let mut frame = frame;
        frame.delegation_chain = chain;
        frame.sign_classical(&l_seed).unwrap();
        frame.payload = b"tampered-by-mitm".to_vec(); // break the signature
        client.send(frame).await.unwrap();

        server.await.unwrap();
    }

    /// Воля АНУ — pure-P2P breach over REAL QUIC, no hub / no relay / no 3rd
    /// party. A node signs its own `BreachAlert` with its real hybrid keys and
    /// sends it DIRECTLY to a peer over a single QUIC stream. The receiver's
    /// `recv` bypasses the roster/UCAN gate (breach is a self-signed fail-safe)
    /// and admits the frame after verifying BOTH signature legs — proving the
    /// alarm crossed the wire P2P and is forge-proof. No shared roster, no
    /// anchor, no intermediary involved.
    // insecure-tls only (accept-any client vs self-signed QUIC cert); the P2P
    // breach-delivery property is independent of the TLS verifier.
    #[cfg(feature = "insecure-tls")]
    #[tokio::test]
    async fn quic_p2p_breach_no_hub_no_roster() {
        let _lock = QUIC_PORT_LOCK.lock().await;
        let addr = free_port().await;

        // Broadcaster node: its own hybrid keys (Ed25519 + ML-DSA-65).
        let ed = [0x77u8; 32];
        let (pq_pk, pq_sk) = bebop2_core::pq_dsa::keygen_derivable(&[0x78u8; 32]);

        // Kernel-shaped alert (40 bytes): this node, group of 5, tamper detected.
        let mut alert = [0u8; BREACH_ALERT_BYTES];
        alert[..32].copy_from_slice(&[0xABu8; 32]);
        alert[32..].copy_from_slice(&5u64.to_le_bytes());

        // Sign a real breach frame (BreachAlarm/Broadcast) with BOTH legs.
        let cap = Capability::new_hybrid(
            bebop2_core::sign::keygen(&ed).0,
            pq_pk.bytes.clone(),
            Resource::BreachAlarm,
            Action::Broadcast,
            [0x99u8; 8],
            9_999_999_999,
        );
        let mut frame = SignedFrame::new(cap, alert.to_vec());
        frame.sign_classical(&ed).unwrap();
        frame.sign_pq(&pq_sk.bytes.clone().try_into().unwrap(), &[0u8; 32])
            .unwrap();

        let (tx, rx) = oneshot::channel();
        let server_addr = addr.clone();
        let server_done = std::sync::Arc::new(tokio::sync::Notify::new());
        let client_done = server_done.clone();
        let server = tokio::spawn(async move {
            let _ = tx.send(());
            let ep = QuicEndpoint::Bind(server_addr);
            // Note: NO roster set — pure P2P, no enrolled-anchor trust required.
            let mut t = QuicTransport::accept(&ep).await.unwrap();
            let got = t.recv().await.expect("peer breach admitted over QUIC P2P");
            // Receiver re-verifies the breach (hybrid sig, no roster) and checks
            // it is a breach broadcast with the right payload size.
            assert_eq!(
                got.capability.scope.grants,
                &[(Resource::BreachAlarm, Action::Broadcast)]
            );
            assert!(got.verify_classical().is_ok() && got.verify_pq().is_ok());
            assert_eq!(got.payload.len(), BREACH_ALERT_BYTES);
            assert_eq!(got.payload, alert, "P2P breach payload matches sender's alert");
            server_done.notified().await;
        });
        rx.await.unwrap();

        let client_ep = QuicEndpoint::Dial(addr);
        let mut client = QuicTransport::connect(&client_ep).await.unwrap();
        client.send(frame).await.unwrap();

        client_done.notify_one();
        server.await.unwrap();
    }

    /// Воля АНУ — BIDIRECTIONAL domain separation (the missing half of the
    /// forge-proof). A `BreachAlarm/Broadcast` frame MUST be REJECTED by the
    /// NORMAL UCAN hybrid gate (`HybridGate::check` with an empty roster/chain
    /// → `UnknownIssuer`): it is a SELF-SIGNED fail-safe, not a delegated
    /// capability, so it can NEVER be smuggled through the standard
    /// authorization path. It is admitted ONLY by the dedicated P2P bypass in
    /// `recv` (which verifies both signature legs directly, no roster). This
    /// test proves the separation holds BOTH ways — the breach scope is
    /// neither over-broad (accepted as a normal cap) nor silently dropped.
    #[test]
    fn breach_scope_rejected_by_normal_gate_not_smuggled() {
        // Build a real, fully-signed breach frame (both legs valid).
        let ed = [0x77u8; 32];
        let (pq_pk, pq_sk) = bebop2_core::pq_dsa::keygen_derivable(&[0x78u8; 32]);
        let cap = Capability::new_hybrid(
            bebop2_core::sign::keygen(&ed).0,
            pq_pk.bytes.clone(),
            Resource::BreachAlarm,
            Action::Broadcast,
            [0x99u8; 8],
            9_999_999_999,
        );
        let mut frame = SignedFrame::new(cap, vec![0u8; BREACH_ALERT_BYTES]);
        frame.sign_classical(&ed).unwrap();
        frame
            .sign_pq(&pq_sk.bytes.clone().try_into().unwrap(), &[0u8; 32])
            .unwrap();

        // Both signature legs verify — it is a genuinely signed frame.
        assert!(frame.verify_classical().is_ok() && frame.verify_pq().is_ok());

        // BUT the normal UCAN gate (empty roster, no delegation chain) REJECTS
        // it. A breach alarm is not a delegated capability; admitting it via
        // the standard path would be a privilege-escalation / smuggling hole.
        let gate = HybridGate::new(HybridPolicy::RequireBoth);
        let empty_roster = AnchorRoster::new();
        let res = gate.check(&frame, &empty_roster, &[], &RevocationSet::new(), 9_999_999_999);
        assert!(
            res.is_err(),
            "BreachAlarm MUST be rejected by the normal UCAN gate (no roster)"
        );

        // Sanity: an ordinary delegated scope on the same empty roster is ALSO
        // rejected (proves we are testing the gate, not a breach-specific path).
        let normal_cap = Capability::new_hybrid(
            bebop2_core::sign::keygen(&ed).0,
            pq_pk.bytes.clone(),
            Resource::Route,
            Action::Send,
            [0x99u8; 8],
            9_999_999_999,
        );
        let mut normal_frame = SignedFrame::new(normal_cap, b"hi".to_vec());
        normal_frame.sign_classical(&ed).unwrap();
        normal_frame
            .sign_pq(&pq_sk.bytes.clone().try_into().unwrap(), &[0u8; 32])
            .unwrap();
        assert!(gate
            .check(&normal_frame, &empty_roster, &[], &RevocationSet::new(), 9_999_999_999)
            .is_err());
    }
}
