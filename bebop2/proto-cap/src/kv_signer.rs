//! Real hybrid (Ed25519 ⊕ ML-DSA-65) K/V split-identity signer.
//!
//! BLUEPRINT-P06 §7.7 — replaces the open crypto slot with a genuine two-leg
//! hybrid signer enforcing **RequireBoth** semantics: a frame/statement is
//! accepted ONLY if BOTH the classical (Ed25519) and the post-quantum
//! (ML-DSA-65, FIPS 204) legs verify. A 1-bit corruption of *either* leg MUST
//! fail verification.
//!
//! # K/V split identity
//! Two distinct roles — `K` and `V` — are derived from ONE master seed under
//! domain-separated labels `b"dowiz.v1.key_K"` / `b"dowiz.v1.key_V"`. Because
//! the labels differ, the role seeds differ, so the Ed25519 AND the ML-DSA-65
//! keypairs for K and V are distinct by construction (K != V). The PQ leg of
//! each role additionally routes through [`bebop2_core::pq_dsa::derive_pq_seed`]
//! (C6 domain separation) so the classical and PQ private keys of a single role
//! are not derived from one raw seed either.
//!
//! ZERO external dependencies (proto-cap is zero-dep by design). Hex is used for
//! all serialization on the wire; no base64 crate is pulled in.

use bebop2_core::pq_dsa::{self, MlDsa65Pk, MlDsa65Sk, MlDsa65Sig};
use bebop2_core::sign;

/// Domain-separation labels for the two roles (K = key-issuer, V = verifier).
pub const LABEL_K: &[u8] = b"dowiz.v1.key_K";
pub const LABEL_V: &[u8] = b"dowiz.v1.key_V";

/// A full K/V keypair (private + public material) for one role.
/// Manual Clone (the PQ keys wrap a `Vec<u8>` but don't derive `Clone`).
pub struct KvKey {
    pub ed_pub: [u8; 32],
    pub ed_priv: [u8; 32],
    pub pq_pk: MlDsa65Pk,
    pub pq_sk: MlDsa65Sk,
}

impl Clone for KvKey {
    fn clone(&self) -> Self {
        KvKey {
            ed_pub: self.ed_pub,
            ed_priv: self.ed_priv,
            pq_pk: MlDsa65Pk {
                bytes: self.pq_pk.bytes.clone(),
            },
            pq_sk: MlDsa65Sk {
                bytes: self.pq_sk.bytes.clone(),
            },
        }
    }
}

/// The public anchor material for one role (serializable to a kv-genesis line).
pub struct KvPub {
    pub ed_pub: [u8; 32],
    pub pq_pk: MlDsa65Pk,
}

impl Clone for KvPub {
    fn clone(&self) -> Self {
        KvPub {
            ed_pub: self.ed_pub,
            pq_pk: MlDsa65Pk {
                bytes: self.pq_pk.bytes.clone(),
            },
        }
    }
}

/// A hybrid signature: both legs. RequireBoth means BOTH must verify.
#[derive(Clone)]
pub struct HybridSig {
    pub ed_sig: [u8; 64],
    pub pq_sig: Vec<u8>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Derivation
// ─────────────────────────────────────────────────────────────────────────────

/// Derive a 32-byte role seed from the master seed + a domain label.
/// `sha3_256(master || label)` — fixed-width master, so unambiguous.
fn derive_role_seed(master: &[u8; 32], label: &[u8]) -> [u8; 32] {
    let mut input = Vec::with_capacity(32 + label.len());
    input.extend_from_slice(master);
    input.extend_from_slice(label);
    bebop2_core::hash::sha3_256(&input)
}

/// Derive one role's full keypair from the master seed and a domain label.
fn derive_kv_key(master: &[u8; 32], label: &[u8]) -> KvKey {
    let role_seed = derive_role_seed(master, label);
    // Classical leg: Ed25519. Per bebop2_core::sign::keygen, the returned
    // second element is the PUBLIC key (`sk = pk`); the true Ed25519 private
    // key IS the seed. So we keep `role_seed` as `ed_priv` and take `ed_pub`
    // from `keygen`. `sign::sign(seed, msg)` consumes the seed directly.
    let (ed_pub, _sk) = sign::keygen(&role_seed);
    let ed_priv = role_seed;
    // PQ leg: C6 domain-separated seed, then ML-DSA-65 keygen.
    let pq_seed = pq_dsa::derive_pq_seed(&role_seed);
    let (pq_pk, pq_sk) = pq_dsa::keygen_derivable(&pq_seed);
    KvKey {
        ed_pub,
        ed_priv,
        pq_sk,
        pq_pk,
    }
}

/// Derive the K and V keypairs from one master seed.
/// Returns `(K, V)`. K != V by construction (distinct domain labels).
pub fn derive_kv_keys(master_seed: &[u8; 32]) -> (KvKey, KvKey) {
    (derive_kv_key(master_seed, LABEL_K), derive_kv_key(master_seed, LABEL_V))
}

/// Public anchor material for a role (drops the secret halves).
pub fn kv_pub(key: &KvKey) -> KvPub {
    KvPub {
        ed_pub: key.ed_pub,
        pq_pk: MlDsa65Pk {
            bytes: key.pq_pk.bytes.clone(),
        },
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Sign / verify
// ─────────────────────────────────────────────────────────────────────────────

/// Sign `msg` with both legs of `key` (deterministic PQ mode: rnd = 0).
pub fn sign_hybrid(key: &KvKey, msg: &[u8]) -> HybridSig {
    let ed_sig = sign::sign(&key.ed_priv, msg);
    // rnd = 0 -> FIPS 204 deterministic signing mode. The deterministic KAT
    // path is what the ACVP gate covers; the operator ceremony may substitute
    // a random rnd in a future variant.
    let rnd = [0u8; 32];
    let pq_sig = pq_dsa::sign(&key.pq_sk, msg, &rnd);
    HybridSig {
        ed_sig,
        pq_sig: pq_sig.bytes,
    }
}

/// Verify a hybrid signature under **RequireBoth**: true ONLY if BOTH legs
/// verify. A corruption of either leg returns false.
pub fn verify_hybrid(kvkey_pub: &KvPub, msg: &[u8], sig: &HybridSig) -> bool {
    let ed_ok = sign::verify(&kvkey_pub.ed_pub, msg, &sig.ed_sig);
    if !ed_ok {
        return false; // fail-closed: no short-circuit that could mask PQ status
    }
    let pq_sig = MlDsa65Sig {
        bytes: sig.pq_sig.clone(),
    };
    let pq_ok = pq_dsa::verify(&kvkey_pub.pq_pk, msg, &pq_sig);
    pq_ok
}

// ─────────────────────────────────────────────────────────────────────────────
// Wire / anchor serialization (hex, zero-dep)
// ─────────────────────────────────────────────────────────────────────────────

impl KvPub {
    /// Produce a kv-genesis anchor line: `<hex(ed_pub||pq_pk)> role=X`.
    /// The hex encodes `ed_pub` (64 hex chars) immediately followed by
    /// `pq_pk.bytes` (the 1952-byte ML-DSA-65 public key), so a parser can
    /// split at the fixed 64-char offset.
    pub fn to_anchor_line(&self, role: char) -> String {
        let mut h = hex_encode(&self.ed_pub);
        h.push_str(&hex_encode(&self.pq_pk.bytes));
        format!("{} role={}", h, role)
    }

    /// Parse an anchor line produced by [`KvPub::to_anchor_line`].
    /// Accepts `role=K` or `role=V`. Returns the parsed public anchor.
    pub fn from_anchor_line(line: &str) -> Option<(KvPub, char)> {
        let line = line.trim();
        let mut parts = line.splitn(2, " role=");
        let hexpart = parts.next()?.trim();
        let rolepart = parts.next()?;
        let role = rolepart.trim().chars().next()?;
        if role != 'K' && role != 'V' {
            return None;
        }
        // ed_pub = first 64 hex chars, pq_pk = the rest.
        if hexpart.len() < 64 {
            return None;
        }
        let (ed_hex, pq_hex) = hexpart.split_at(64);
        let ed_pub = hex_decode(ed_hex).ok()?;
        if ed_pub.len() != 32 {
            return None;
        }
        let mut ed_arr = [0u8; 32];
        ed_arr.copy_from_slice(&ed_pub);
        let pq_bytes = hex_decode(pq_hex).ok()?;
        Some((
            KvPub {
                ed_pub: ed_arr,
                pq_pk: MlDsa65Pk { bytes: pq_bytes },
            },
            role,
        ))
    }
}

/// Encode a hybrid signature to hex: `ed_sig` (128 hex) ++ `pq_sig`.
pub fn sig_to_hex(sig: &HybridSig) -> String {
    let mut h = hex_encode(&sig.ed_sig);
    h.push_str(&hex_encode(&sig.pq_sig));
    h
}

/// Decode a hex hybrid signature produced by [`sig_to_hex`].
pub fn sig_from_hex(s: &str) -> Option<HybridSig> {
    if s.len() < 128 {
        return None;
    }
    let (ed_hex, pq_hex) = s.split_at(128);
    let ed = hex_decode(ed_hex).ok()?;
    if ed.len() != 64 {
        return None;
    }
    let mut ed_sig = [0u8; 64];
    ed_sig.copy_from_slice(&ed);
    let pq_sig = hex_decode(pq_hex).ok()?;
    Some(HybridSig { ed_sig, pq_sig })
}

// ─────────────────────────────────────────────────────────────────────────────
// Tiny zero-dep hex codec
// ─────────────────────────────────────────────────────────────────────────────

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0f) as usize] as char);
    }
    out
}

fn hex_decode(s: &str) -> Result<Vec<u8>, &'static str> {
    let s = s.as_bytes();
    if s.len() % 2 != 0 {
        return Err("odd hex length");
    }
    let mut out = Vec::with_capacity(s.len() / 2);
    let mut i = 0;
    while i < s.len() {
        let hi = hex_val(s[i])?;
        let lo = hex_val(s[i + 1])?;
        out.push((hi << 4) | lo);
        i += 2;
    }
    Ok(out)
}

fn hex_val(c: u8) -> Result<u8, &'static str> {
    match c {
        b'0'..=b'9' => Ok(c - b'0'),
        b'a'..=b'f' => Ok(c - b'a' + 10),
        b'A'..=b'F' => Ok(c - b'A' + 10),
        _ => Err("invalid hex char"),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests — falsifiable (BLUEPRINT-P06 §7.6 / §7.7)
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // FIXED test master seed — test-only, NOT a committed trust root. The
    // operator mints the real kv-genesis.txt from a separate high-entropy seed.
    const TEST_MASTER: [u8; 32] = [0x42u8; 32];

    #[test]
    fn deterministic_keygen_k_ne_v() {
        let (k1, v1) = derive_kv_keys(&TEST_MASTER);
        let (k2, v2) = derive_kv_keys(&TEST_MASTER);
        // deterministic: same master -> same anchors
        assert_eq!(k1.ed_pub, k2.ed_pub, "K ed_pub must be deterministic");
        assert_eq!(v1.ed_pub, v2.ed_pub, "V ed_pub must be deterministic");
        assert_eq!(k1.pq_pk.bytes, k2.pq_pk.bytes, "K pq_pk deterministic");
        assert_eq!(v1.pq_pk.bytes, v2.pq_pk.bytes, "V pq_pk deterministic");
        // K != V by construction
        assert_ne!(k1.ed_pub, v1.ed_pub, "K and V Ed25519 pubkeys must differ");
        assert_ne!(
            k1.pq_pk.bytes, v1.pq_pk.bytes,
            "K and V ML-DSA-65 pubkeys must differ"
        );
    }

    #[test]
    fn roundtrip_k_and_v() {
        let (k, v) = derive_kv_keys(&TEST_MASTER);
        let msg = b"dowiz v1 split-identity verifier genesis";
        // K signs, K verifies
        let sig_k = sign_hybrid(&k, msg);
        assert!(
            verify_hybrid(&kv_pub(&k), msg, &sig_k),
            "K roundtrip must verify"
        );
        // V signs, V verifies
        let sig_v = sign_hybrid(&v, msg);
        assert!(
            verify_hybrid(&kv_pub(&v), msg, &sig_v),
            "V roundtrip must verify"
        );
        // anchor line roundtrips
        let line = kv_pub(&k).to_anchor_line('K');
        let (parsed, role) = KvPub::from_anchor_line(&line).expect("parse anchor");
        assert_eq!(role, 'K');
        assert_eq!(parsed.ed_pub, k.ed_pub);
        assert_eq!(parsed.pq_pk.bytes, k.pq_pk.bytes);
        // sig hex roundtrips
        let sig_hex = sig_to_hex(&sig_k);
        let sig_back = sig_from_hex(&sig_hex).expect("parse sig hex");
        assert!(verify_hybrid(&kv_pub(&k), msg, &sig_back), "sig hex roundtrip");
    }

    #[test]
    fn one_bit_corruption_ed_sig_fails() {
        let (k, _v) = derive_kv_keys(&TEST_MASTER);
        let msg = b"corrupt-the-classical-leg";
        let mut sig = sign_hybrid(&k, msg);
        assert!(verify_hybrid(&kv_pub(&k), msg, &sig), "precondition: clean sig ok");
        // flip 1 bit in the Ed25519 signature
        sig.ed_sig[0] ^= 0x01;
        assert!(
            !verify_hybrid(&kv_pub(&k), msg, &sig),
            "1-bit corruption of ed_sig MUST fail"
        );
    }

    #[test]
    fn one_bit_corruption_pq_sig_fails() {
        let (k, _v) = derive_kv_keys(&TEST_MASTER);
        let msg = b"corrupt-the-pq-leg";
        let mut sig = sign_hybrid(&k, msg);
        assert!(verify_hybrid(&kv_pub(&k), msg, &sig), "precondition: clean sig ok");
        // flip 1 bit in the ML-DSA-65 signature
        let last = sig.pq_sig.len() - 1;
        sig.pq_sig[last] ^= 0x80;
        assert!(
            !verify_hybrid(&kv_pub(&k), msg, &sig),
            "1-bit corruption of pq_sig MUST fail"
        );
    }

    #[test]
    fn wrong_role_anchor_fails() {
        let (k, v) = derive_kv_keys(&TEST_MASTER);
        let msg = b"role-separation at verify";
        // V signs the message...
        let sig_v = sign_hybrid(&v, msg);
        // ...but we verify it under K's anchor. Must fail (defense in depth:
        // a V-signed message is not accepted under K's public key, and vice
        // versa — the two identities are cryptographically disjoint).
        assert!(
            !verify_hybrid(&kv_pub(&k), msg, &sig_v),
            "V-signed msg MUST NOT verify under K's anchor"
        );
        assert!(
            verify_hybrid(&kv_pub(&v), msg, &sig_v),
            "sanity: V-signed msg verifies under V's anchor"
        );
    }
}
