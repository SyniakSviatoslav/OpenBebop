//! At-rest encryption layer (P09 §6 · F30) — XChaCha20-Poly1305, zero-dep.
//!
//! This is the **same** AEAD as the wire session encryptor (`crate::aead`),
//! reviewed once, used on two surfaces: confidentiality-in-transit (F16) and
//! confidentiality-at-rest (F30). The two surfaces are isolated by **distinct
//! keys** (a per-hub at-rest key vs the per-session wire key) and **distinct
//! nonce domains** (the at-rest domain is prefixed `b"bebop::atrest\0"` so an
//! at-rest ciphertext can never be confused with a wire ciphertext, and a nonce
//! reused in one domain is not a reuse in the other). This satisfies the
//! blueprint's "one AEAD across the system … distinct keys and nonce domains"
//! requirement and the acceptance criterion "no nonce reuse across surfaces".
//!
//! innovate: WHY REUSE THE ZERO-DEP CORE CIPHER INSTEAD OF THE LEGACY
//! `crates/bebop/src/vault.rs`:
//!   The blueprint (§6.1) noted a real at-rest layer exists in the LEGACY
//!   `crates/bebop/src/vault.rs` (Argon2id + ML-KEM⊕X25519 + XChaCha20-Poly1305).
//!   That vault calls the RustCrypto `chacha20poly1305` crate. Importing it into
//!   `bebop2` would (a) add a new dependency (violating `Cargo.lock`-must-stay-
//!   unchanged), and (b) pull an AGPL-3.0 crate into the MIT-licensed,
//!   empty-import, zero-dep `bebop2-core` — a dependency-direction violation
//!   (the live per-hub store may not reach back into the legacy crate). The
//!   blueprint's own fallback rule (§6.2) says: if the legacy AEAD cannot be
//!   cleanly imported, relocate/vendor the MINIMAL zero-dep AEAD core into
//!   `bebop2-core`. That core ALREADY EXISTS here as `crate::aead` (RFC 8439 /
//!   draft-irtf-cfrg-xchacha-03, verified against the published KATs). So F30
//!   reuses `crate::aead` directly — no relocation needed, no new dep, the wasm
//!   empty-import guarantee is preserved, and the wire + at-rest layers share
//!   one reviewed implementation exactly as the blueprint demands.
//!
//! KEY MANAGEMENT (F30 LOCK qualifier "EnvFile-key"): the at-rest key derives
//! from an EnvFile secret via Argon2id (`crate::kdf`), never in-repo, so the key
//! stays gitleaks-clean. A caller supplies the 32-byte key; `AtRestStore` never
//! constructs or stores the raw secret. `derive_key_from_env_secret` is the
//! provided bridge.
//!
//! NONCE POLICY (never reuse per key): every record is sealed with a fresh,
//! 24-byte nonce drawn from real platform entropy (`crate::rng::entropy_provider`).
//! The nonce is stored in the clear alongside each ciphertext (standard AEAD
//! practice; XChaCha20's 192-bit nonce space makes entropy-exhaustion
//! infeasible). The nonce domain prefix below is mixed into the AEAD `aad` so
//! at-rest and wire frames are cryptographically disjoint even though they share
//! the same primitive.

use alloc::vec::Vec;

use crate::aead::{aead_xchacha20_poly1305_decrypt, aead_xchacha20_poly1305_encrypt};
use crate::event_log::EventLog;
use crate::hash::sha3_256;

/// AAD nonce-domain separator. Distinct from the wire domain (`b"bebop::galley"`
/// used in `aead.rs`'s tests) so at-rest ciphertexts are cryptographically
/// disjoint from wire session ciphertexts even under a key mix-up.
const AT_REST_DOMAIN: &[u8] = b"bebop::atrest\0";

/// Layout of one persisted record on disk (length-prefixed, NOT serde):
///   nonce:  24 bytes  (XChaCha20 nonce, public)
///   tag:    16 bytes  (Poly1305 tag, public)
///   ct:     remaining (ciphertext, same length as plaintext)
/// The whole record is the payload stored inside the host `EventLog`.
const NONCE_LEN: usize = 24;
const TAG_LEN: usize = 16;
const RECORD_OVERHEAD: usize = NONCE_LEN + TAG_LEN;

/// Errors returned by the at-rest store. All are fail-closed: a corrupt or
/// tampered record yields `Err`, never a plaintext fallback.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AtRestError {
    /// AEAD tag check failed (tamper / wrong key / bit flip). No plaintext is
    /// returned.
    DecryptFailed,
    /// The on-disk record bytes were malformed (too short to carry a nonce+tag).
    MalformedRecord,
    /// `std`-only disk I/O failed (propagates the OS error string).
    Io(String),
}

/// A per-hub at-rest store: an append-only, hash-chained [`EventLog`] whose
/// payloads are **ciphertext** rather than plaintext.
///
/// - `append_encrypted` seals `plaintext` with the per-hub key and a fresh
///   nonce, then stores the resulting `(nonce || tag || ct)` record in the log.
/// - `read_decrypted` fetches the stored record at `seq` and opens it.
/// - `verify` re-checks the log's SHA3-256 hash chain (integrity of the
///   ciphertext envelope) — tampering with stored bytes breaks the chain.
///
/// The struct is generic over a phantom event-domain tag `E` so an at-rest log
/// of one event kind cannot be accidentally mixed with another (same type-safety
/// as the host `EventLog`).
pub struct AtRestStore<E = ()> {
    log: EventLog<E>,
    /// Per-hub at-rest key (32 bytes). Supplied by the caller (e.g. derived
    /// from an EnvFile secret). Held in memory only; never persisted here.
    key: [u8; 32],
}

impl<E> AtRestStore<E> {
    /// Open a store over an existing hash-chained log and a per-hub key.
    pub fn new(log: EventLog<E>, key: [u8; 32]) -> Self {
        AtRestStore { log, key }
    }

    /// The per-hub at-rest key (e.g. for re-derivation checks). Never written to
    /// disk by this layer.
    pub fn key(&self) -> &[u8; 32] {
        &self.key
    }

    /// Seal `plaintext` with the per-hub key + a FRESH 24-byte nonce and append
    /// the `(nonce || tag || ct)` record to the log. Returns the assigned
    /// monotonic sequence number. `key` is captured by the closure at seal time
    /// so the nonce can be drawn from live platform entropy.
    pub fn append_encrypted(&mut self, plaintext: &[u8]) -> u64 {
        let key = self.key;
        let mut nonce = [0u8; NONCE_LEN];
        crate::rng::entropy_provider()
            .fill(&mut nonce)
            .expect("platform entropy unavailable — at-rest seal is fail-closed");

        let (ct, tag) = aead_xchacha20_poly1305_encrypt(&key, &nonce, plaintext, AT_REST_DOMAIN);

        let mut record = Vec::with_capacity(RECORD_OVERHEAD + ct.len());
        record.extend_from_slice(&nonce);
        record.extend_from_slice(&tag);
        record.extend_from_slice(&ct);
        self.log.append(&record)
    }

    /// Number of records currently stored.
    pub fn len(&self) -> usize {
        self.log.len()
    }

    /// Whether the store currently holds zero records.
    pub fn is_empty(&self) -> bool {
        self.log.is_empty()
    }

    /// Open the record at `seq`: fetch the stored `(nonce || tag || ct)` from the
    /// log and run the AEAD decrypt. Returns `None` on a tampered record, a
    /// wrong key, or an out-of-range `seq` (fail-closed). The returned `Vec<u8>`
    /// is the original plaintext.
    pub fn read_decrypted(&self, seq: u64) -> Result<Vec<u8>, AtRestError> {
        let record = self.log.payload_at(seq).ok_or(AtRestError::DecryptFailed)?;
        if record.len() < RECORD_OVERHEAD {
            return Err(AtRestError::MalformedRecord);
        }
        let mut nonce = [0u8; NONCE_LEN];
        nonce.copy_from_slice(&record[..NONCE_LEN]);
        let mut tag = [0u8; TAG_LEN];
        tag.copy_from_slice(&record[NONCE_LEN..RECORD_OVERHEAD]);
        let ct = &record[RECORD_OVERHEAD..];

        aead_xchacha20_poly1305_decrypt(&self.key, &nonce, ct, &tag, AT_REST_DOMAIN)
            .ok_or(AtRestError::DecryptFailed)
    }

    /// Re-verify the underlying hash chain. Catches any tampering with the stored
    /// ciphertext envelope (the SHA3-256 spine covers every stored payload).
    pub fn verify(&self) -> Result<(), crate::event_log::EventLogError> {
        self.log.verify()
    }

    /// Rolling hash of the tip of the (ciphertext) log.
    pub fn root_hash(&self) -> [u8; 32] {
        self.log.root_hash()
    }

    /// Borrow the raw stored RECORD bytes at `seq` (ciphertext, for on-disk
    /// inspection / persistence). Exposed so a disk-backed wrapper can serialize
    /// exactly what is stored without re-encrypting.
    pub fn raw_record_at(&self, seq: u64) -> Option<&[u8]> {
        self.log.payload_at(seq)
    }
}

// ── std-gated disk persistence (the Phase-12 BlockStore integration point) ─────
// Gated behind `feature = "std"` so the pure no_std crypto core stays free of
// `std::fs`. When the durable `BlockStore` lands (Phase 12), it wraps these two
// functions: `persist` writes the ciphertext log to a single file (so the on-disk
// bytes are ciphertext, never plaintext), and `restore` re-opens it.
#[cfg(feature = "std")]
impl<E> AtRestStore<E> {
    /// Serialize the whole ciphertext log to `path` as raw length-prefixed
    /// records (plaintext never touches disk). Format per record:
    ///   u64 LE length || (nonce[24] || tag[16] || ct[..])
    /// The whole file is ciphertext; a reader confirms the on-disk bytes differ
    /// from what was appended.
    pub fn persist(&self, path: &std::path::Path) -> Result<(), AtRestError> {
        use std::fs::File;
        use std::io::{BufWriter, Write};
        let f = File::create(path).map_err(|e| AtRestError::Io(e.to_string()))?;
        let mut w = BufWriter::new(f);
        for seq in 0..(self.log.len() as u64) {
            let rec = self
                .log
                .payload_at(seq)
                .ok_or(AtRestError::MalformedRecord)?;
            w.write_all(&(rec.len() as u64).to_le_bytes())
                .map_err(|e| AtRestError::Io(e.to_string()))?;
            w.write_all(rec)
                .map_err(|e| AtRestError::Io(e.to_string()))?;
        }
        w.flush().map_err(|e| AtRestError::Io(e.to_string()))?;
        Ok(())
    }

    /// Read a ciphertext log written by [`AtRestStore::persist`] and rebuild the
    /// hash chain. `key` must match the key used at seal time (else every record
    /// fails to decrypt). Returns the reopened store.
    pub fn restore(path: &std::path::Path, key: [u8; 32]) -> Result<Self, AtRestError> {
        use std::fs::File;
        use std::io::{BufReader, Read};
        let f = File::open(path).map_err(|e| AtRestError::Io(e.to_string()))?;
        let mut r = BufReader::new(f);
        let mut payloads: Vec<Vec<u8>> = Vec::new();
        loop {
            let mut len_buf = [0u8; 8];
            match r.read_exact(&mut len_buf) {
                Ok(()) => {}
                Err(ref e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
                Err(e) => return Err(AtRestError::Io(e.to_string())),
            }
            let len = u64::from_le_bytes(len_buf) as usize;
            let mut rec = vec![0u8; len];
            r.read_exact(&mut rec)
                .map_err(|e| AtRestError::Io(e.to_string()))?;
            payloads.push(rec);
        }
        let log = EventLog::rebuild_from_payloads(&payloads);
        Ok(AtRestStore::new(log, key))
    }
}

/// Derive the 32-byte at-rest key from an EnvFile secret via Argon2id
/// (RFC 9106). The `secret` is never written to disk by this function; the
/// caller is responsible for sourcing it from an environment file (never
/// in-repo). `hub_id` is bound into the KDF salt + AAD domain so different hubs
/// get distinct keys and a record from one hub cannot be opened under another's
/// key (key-isolation across hubs).
pub fn derive_key_from_env_secret(hub_id: &[u8], secret: &[u8]) -> [u8; 32] {
    let mut salt = [0u8; 16];
    // Bind the hub identity into the Argon2id salt so the derived key is
    // hub-specific (key isolation). SHA3-256(hub_id) → 16-byte salt.
    let h = sha3_256(hub_id);
    salt.copy_from_slice(&h[..16]);
    let tag = crate::kdf::argon2id(secret, &salt, &[], AT_REST_DOMAIN, 3, 32, 4, 32);
    let mut key = [0u8; 32];
    key.copy_from_slice(&tag[..32]);
    key
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key() -> [u8; 32] {
        [0x42u8; 32]
    }

    #[test]
    fn at_rest_encrypt_on_write_and_decrypt_on_read_roundtrip() {
        let mut store = AtRestStore::<()>::new(EventLog::new(), key());
        let secret = b"the cosmo-noir helm turns by starlight, never by panic.";

        let seq = store.append_encrypted(secret);
        assert_eq!(seq, 0);
        assert_eq!(store.len(), 1);

        // On-disk (stored) bytes must NOT be the plaintext.
        let record = store.raw_record_at(0).unwrap();
        assert!(
            record.len() >= RECORD_OVERHEAD,
            "record carries nonce+tag+ct"
        );
        assert_ne!(
            record, secret,
            "RED: stored record must be ciphertext, not plaintext"
        );
        // The plaintext must not appear anywhere in the stored record.
        assert!(
            !record.windows(secret.len()).any(|w| w == secret),
            "RED: plaintext substring must not leak into the stored record"
        );
        // A Poly1305 tag is present (16 bytes after the 24-byte nonce).
        let tag = &record[NONCE_LEN..RECORD_OVERHEAD];
        assert_ne!(tag, &[0u8; 16], "a real tag must be present");

        // Read it back: decrypt must recover the exact plaintext.
        let opened = store
            .read_decrypted(0)
            .expect("GREEN: decrypt must recover plaintext");
        assert_eq!(opened, secret, "round-trip must be byte-identical");

        // The hash chain over the ciphertext envelope stays verifiable.
        assert!(store.verify().is_ok());
    }

    #[test]
    fn at_rest_wrong_key_fails_to_decrypt() {
        let mut store = AtRestStore::<()>::new(EventLog::new(), key());
        store.append_encrypted(b"classified at-rest payload");

        // A store under a different (wrong) key must fail closed on read.
        let reopened_log = EventLog::rebuild_from_payloads(&store.log.snapshot_payloads());
        let wrong = AtRestStore::<()>::new(reopened_log, [0u8; 32]);
        let res = wrong.read_decrypted(0);
        assert!(
            matches!(res, Err(AtRestError::DecryptFailed)),
            "RED: wrong key must be rejected (fail-closed, no plaintext)"
        );
    }

    #[test]
    fn at_rest_tampered_record_fails_tag_check() {
        let mut store = AtRestStore::<()>::new(EventLog::new(), key());
        store.append_encrypted(b"integrity-bound at-rest record");

        // Flip one ciphertext byte in the stored record (simulating on-disk
        // tamper) and reopen under the same key: the tag check must reject it.
        let mut rec = store.raw_record_at(0).unwrap().to_vec();
        let last = rec.len() - 1;
        rec[last] ^= 0x01;
        let tampered = EventLog::rebuild_from_payloads(&[rec]);
        let reopened = AtRestStore::<()>::new(tampered, key());
        assert!(
            matches!(reopened.read_decrypted(0), Err(AtRestError::DecryptFailed)),
            "RED: tampered ciphertext must fail the Poly1305 tag check"
        );
    }

    #[test]
    fn at_rest_nonces_unique_per_record() {
        // Two records under the same key must use DIFFERENT nonces (never reuse
        // per key — the blueprint's core requirement).
        let mut store = AtRestStore::<()>::new(EventLog::new(), key());
        store.append_encrypted(b"record one");
        store.append_encrypted(b"record two");
        let n1 = &store.raw_record_at(0).unwrap()[..NONCE_LEN];
        let n2 = &store.raw_record_at(1).unwrap()[..NONCE_LEN];
        assert_ne!(
            n1, n2,
            "RED: nonce must differ per record under the same key"
        );
    }
}
