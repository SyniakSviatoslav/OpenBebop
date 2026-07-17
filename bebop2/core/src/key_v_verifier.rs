//! BOUNDED key_V verdict verify-leg (Layer E2, CORE-ROADMAP).
//!
//! Verify-only. Reuses the in-tree ACVP-verified ML-DSA-65 engine
//! ([`crate::pq_dsa::verify_internal_bytes`]) — no ML-DSA re-implementation, no
//! signing, no keygen ceremony, no merge-gate, no FalseClaimMeter.
//!
//! A verdict is a canonical TLV (BLUEPRINT-P06 §4) signed with the **key_V**
//! (verifier) ML-DSA-65 private key. The verifier checks the signature against
//! the [`KeyVPublicKey`] ONLY; it never consults any key_K attestation, so a
//! verdict signed by the author's key_K is rejected (wrong-key). Fail-closed:
//! malformed TLV, unknown tag, wrong length, non-UTF-8 rationale, or bad
//! signature all yield `false`; the verifier never panics on untrusted input.

use alloc::vec::Vec;

use crate::pq_dsa::{self, MlDsa65Pk, PUBLICKEYBYTES};

/// Role-typed wrapper for the key_V (verifier) ML-DSA-65 public key.
///
/// The type encodes the split-identity invariant: a verdict verifier accepts
/// ONLY a `KeyVPublicKey`. There is deliberately no constructor that lifts an
/// arbitrary [`MlDsa65Pk`] or a key_K into this type without explicitly naming
/// it `key_V`, so call sites cannot accidentally verify a verdict against the
/// author's key_K. (Cryptographically the engine is symmetric — the guarantee
/// comes from the caller passing the *verifier* anchor and the verifier never
/// touching key_K at all.)
pub struct KeyVPublicKey(pub MlDsa65Pk);

impl KeyVPublicKey {
    /// Wrap raw ML-DSA-65 public-key bytes as the key_V role. Fails closed:
    /// wrong length => `None` (never panics).
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() != PUBLICKEYBYTES {
            return None;
        }
        Some(KeyVPublicKey(MlDsa65Pk {
            bytes: bytes.to_vec(),
        }))
    }
}

// TLV tag whitelist — the canonical P06 §4 verdict shape.
const TAG_VERDICT: u8 = 0x03;
const TAG_SUITE_RESULTS: u8 = 0x04;
const TAG_RATIONALE: u8 = 0x07;

// Other known §4 tags (optional, length-bounded). Anything outside this set is
// an UNKNOWN tag => the verdict is malformed => rejected.
const KNOWN_TAGS: &[u8] = &[0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09];

// Bounds (fail-closed; "bounded" per P06 §4).
const MAX_RATIONALE_LEN: usize = 4096;
const MAX_SUITE_RESULTS_LEN: usize = 65535;
const MAX_OTHER_LEN: usize = 65535;

/// A parsed, well-formed key_V verdict (P06 §4 shape, E2 required slice).
pub struct Verdict<'a> {
    /// 0x00 = RED, 0x01 = GREEN.
    pub verdict: u8,
    /// `T=0x04` suite_results (raw bytes; caller interprets per suite schema).
    pub suite_results: &'a [u8],
    /// `T=0x07` rationale — guaranteed valid UTF-8, length-bounded.
    pub rationale: &'a str,
}

/// Fail-closed TLV parse of a verdict. Returns `None` on ANY malformation
/// (truncation, unknown tag, wrong length, non-UTF-8 rationale, missing
/// required tag). Never panics.
///
/// TLV wire format: `[tag:u8][len:u16 BE][value:len bytes]`, one record per tag.
/// The E2-required tags are `0x03` (1 byte, 0x00/0x01), `0x04` (var), `0x07`
/// (var, valid UTF-8). Other §4 tags are known but optional.
fn parse_verdict(buf: &[u8]) -> Option<Verdict<'_>> {
    let mut i = 0usize;
    let mut have_verdict = false;
    let mut have_suite = false;
    let mut have_rationale = false;
    let mut verdict = 0u8;
    let mut suite_results: &[u8] = &[];
    let mut rationale: &str = "";

    while i < buf.len() {
        let tag = buf[i];
        i += 1;
        // 2-byte big-endian length (supports >255 rationale/suite payloads).
        if i + 2 > buf.len() {
            return None; // truncated length field
        }
        let len = u16::from_be_bytes([buf[i], buf[i + 1]]) as usize;
        i += 2;
        if i + len > buf.len() {
            return None; // declared length runs past end of buffer
        }
        let value = &buf[i..i + len];
        i += len;

        if !KNOWN_TAGS.contains(&tag) {
            return None; // unknown tag
        }
        match tag {
            TAG_VERDICT => {
                if len != 1 {
                    return None; // wrong length
                }
                let v = value[0];
                if v != 0x00 && v != 0x01 {
                    return None; // verdict must be RED(0) or GREEN(1)
                }
                verdict = v;
                have_verdict = true;
            }
            TAG_SUITE_RESULTS => {
                if len > MAX_SUITE_RESULTS_LEN {
                    return None;
                }
                suite_results = value;
                have_suite = true;
            }
            TAG_RATIONALE => {
                if len > MAX_RATIONALE_LEN {
                    return None; // bounded
                }
                match core::str::from_utf8(value) {
                    Ok(s) => {
                        rationale = s;
                        have_rationale = true;
                    }
                    Err(_) => return None, // invalid UTF-8
                }
            }
            _ => {
                // Known §4 tag, not part of the E2-required minimum, length-bounded.
                if len > MAX_OTHER_LEN {
                    return None;
                }
            }
        }
    }

    if !(have_verdict && have_suite && have_rationale) {
        return None; // required tags missing
    }
    Some(Verdict {
        verdict,
        suite_results,
        rationale,
    })
}

/// Verify a key_V-signed verdict, fail-closed.
///
/// * `key_v`      — the verifier's (V-role) ML-DSA-65 public key.
/// * `verdict_tlv` — the canonical TLV-encoded verdict bytes (the signed message).
/// * `sig`        — the ML-DSA-65 signature over `verdict_tlv`.
///
/// Returns `true` iff (a) the TLV parses into a well-formed P06 §4 verdict AND
/// (b) the signature verifies against `key_v`. Both checks are independent of
/// key_K. Any malformation or signature failure yields `false`; never panics.
pub fn verify_key_v_verdict(key_v: &KeyVPublicKey, verdict_tlv: &[u8], sig: &[u8]) -> bool {
    // (a) well-formed TLV verdict (fail-closed).
    if parse_verdict(verdict_tlv).is_none() {
        return false;
    }
    // (b) signature check against key_V ONLY — never consults key_K.
    pq_dsa::verify_internal_bytes(&key_v.0.bytes, verdict_tlv, sig)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pq_dsa::{keygen_bytes, sign_internal_bytes};

    const RND: &[u8; 32] = &[0x42u8; 32];

    fn enc_tlv(buf: &mut Vec<u8>, tag: u8, value: &[u8]) {
        buf.push(tag);
        buf.extend_from_slice(&(value.len() as u16).to_be_bytes());
        buf.extend_from_slice(value);
    }

    fn build_verdict(verdict_byte: u8, suite: &[u8], rationale: &str) -> Vec<u8> {
        let mut v = Vec::new();
        enc_tlv(&mut v, 0x03, &[verdict_byte]);
        enc_tlv(&mut v, 0x04, suite);
        enc_tlv(&mut v, 0x07, rationale.as_bytes());
        v
    }

    // Two DISTINCT seeds => distinct key_K / key_V keypairs.
    fn keypair(seed_byte: u8) -> (Vec<u8>, Vec<u8>) {
        keygen_bytes(&[seed_byte; 32])
    }

    #[test]
    fn red_green_tampered_verdict_fails() {
        // (d) correctly-signed key_V verdict verifies true.
        let (pk_v, sk_v) = keypair(0xA1);
        let kv = KeyVPublicKey::from_bytes(&pk_v).unwrap();
        let verdict = build_verdict(0x01, b"kernel=pass,engine=pass", "all suites green");
        let sig = sign_internal_bytes(&sk_v, &verdict, RND);
        assert!(verify_key_v_verdict(&kv, &verdict, &sig), "valid V verdict must verify");

        // (a) tampered verdict (flip a rationale byte): signature no longer matches.
        let mut tampered = verdict.clone();
        let last = tampered.len() - 1;
        tampered[last] ^= 0xFF;
        assert!(
            !verify_key_v_verdict(&kv, &tampered, &sig),
            "tampered verdict must be rejected"
        );
    }

    #[test]
    fn red_green_wrong_key_rejected() {
        // Sign with key_K; verify against key_V pk => rejected (independent verify).
        let (pk_v, _sk_v) = keypair(0xA1);
        let (pk_k, sk_k) = keypair(0xB2);
        let kv = KeyVPublicKey::from_bytes(&pk_v).unwrap();
        let verdict = build_verdict(0x01, b"kernel=pass", "verdict by K, not V");
        let sig_by_k = sign_internal_bytes(&sk_k, &verdict, RND);
        assert!(
            !verify_key_v_verdict(&kv, &verdict, &sig_by_k),
            "verdict signed by key_K must be rejected against key_V"
        );
        // Sanity: the same signature DOES verify against the true key_K pk.
        assert!(
            pq_dsa::verify_internal_bytes(&pk_k, &verdict, &sig_by_k),
            "control: key_K signature must verify under key_K"
        );
    }

    #[test]
    fn red_green_malformed_tlv_no_panic() {
        let (pk_v, _sk_v) = keypair(0xA1);
        let kv = KeyVPublicKey::from_bytes(&pk_v).unwrap();

        // Truncated mid-record.
        let mut trunc = build_verdict(0x01, b"x", "y");
        trunc.truncate(trunc.len() - 3);
        assert!(!verify_key_v_verdict(&kv, &trunc, &[0u8; 32]), "truncated TLV rejected");

        // Unknown tag injected.
        let mut unknown = Vec::new();
        enc_tlv(&mut unknown, 0x03, &[0x01]);
        enc_tlv(&mut unknown, 0x04, b"x");
        enc_tlv(&mut unknown, 0x07, b"y");
        enc_tlv(&mut unknown, 0xFF, b"evil"); // unknown tag
        assert!(!verify_key_v_verdict(&kv, &unknown, &[0u8; 32]), "unknown tag rejected");

        // Invalid UTF-8 in rationale.
        let mut bad_utf8 = Vec::new();
        enc_tlv(&mut bad_utf8, 0x03, &[0x01]);
        enc_tlv(&mut bad_utf8, 0x04, b"x");
        enc_tlv(&mut bad_utf8, 0x07, &[0xFF, 0xFE]); // not valid UTF-8
        assert!(!verify_key_v_verdict(&kv, &bad_utf8, &[0u8; 32]), "non-UTF-8 rationale rejected");

        // Missing required tag (no 0x03 verdict).
        let mut missing = Vec::new();
        enc_tlv(&mut missing, 0x04, b"x");
        enc_tlv(&mut missing, 0x07, b"y");
        assert!(!verify_key_v_verdict(&kv, &missing, &[0u8; 32]), "missing verdict tag rejected");
    }

    #[test]
    fn red_green_parse_known_optional_tags_ok() {
        // A full §4 verdict (with optional known tags) still parses + verifies.
        let (pk_v, sk_v) = keypair(0xA1);
        let kv = KeyVPublicKey::from_bytes(&pk_v).unwrap();
        let mut v = Vec::new();
        enc_tlv(&mut v, 0x01, &[0x11; 32]); // diff_attest_sha3 (known, optional)
        enc_tlv(&mut v, 0x02, &[0x22; 32]); // recomputed_diff_sha3 (known, optional)
        enc_tlv(&mut v, 0x03, &[0x01]); // GREEN
        enc_tlv(&mut v, 0x04, b"kernel=pass");
        enc_tlv(&mut v, 0x05, &[0x33; 32]); // key_V_anchor_id (known, optional)
        enc_tlv(&mut v, 0x07, b"green, all good");
        enc_tlv(&mut v, 0x09, b"enforced approximation: identity != person");
        let sig = sign_internal_bytes(&sk_v, &v, RND);
        assert!(
            verify_key_v_verdict(&kv, &v, &sig),
            "full §4 verdict with optional tags must verify"
        );
    }
}
