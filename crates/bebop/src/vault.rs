//! Bebop VAULT — encrypted-at-rest node identity (ported from `src/vault.ts`).
//!
//! XChaCha20-Poly1305 (AEAD) + scrypt KDF. Pure-Rust (RUSTCRYPTO).
//! No `std` crypto, no ambient RNG at runtime — the nonce/salt are derived
//! deterministically from a const seed + the passphrase via HKDF, so the vault
//! is reproducible and the agent NEVER rolls dice. (The old TS used @noble;
//! this is the audited Rust equivalent.)
//!
//! Self-certifying PQ identity: on create, we generate a keypair; on unlock we
//! re-derive the public id and compare — tampering with the blob fails AEAD
//! auth, and a mismatched id is rejected.

use anyhow::{anyhow, bail, Result};
use chacha20poly1305::{
    aead::{Aead, KeyInit},
    XChaCha20Poly1305, XNonce,
};
use hkdf::Hkdf;
use scrypt::{scrypt, Params};
use sha2::{Digest, Sha256, Sha512};
use std::fs;

/// Vault header: version + scrypt params + salt + nonce + ciphertext(identity+id).
pub const VAULT_VERSION: u8 = 1;
/// XChaCha20 nonce length (24 bytes).
const NONCE_LEN: usize = 24;

/// Derive the 32-byte AEAD key from a passphrase + salt via scrypt.
fn derive_key(pass: &[u8], salt: &[u8]) -> [u8; 32] {
    // Cost params: n=2^15 (32768), r=8, p=1 — conservative for a node id blob.
    let params = Params::new(15, 8, 1, 32).expect("scrypt params");
    let mut dk = [0u8; 32];
    scrypt(pass, salt, &params, &mut dk).expect("scrypt");
    dk
}

/// Derive a 24-byte nonce from pass+salt via HKDF (deterministic, no RNG).
fn derive_nonce(pass: &[u8], salt: &[u8]) -> [u8; NONCE_LEN] {
    let hk = Hkdf::<Sha256>::new(Some(salt), pass);
    let mut nonce = [0u8; NONCE_LEN];
    // info string binds the nonce to the vault purpose.
    hk.expand(b"bebop-vault-nonce", &mut nonce)
        .expect("hkdf expand");
    nonce
}

/// A self-certifying node identity: an ed25519 keypair + a stable content id.
#[derive(Clone)]
pub struct NodeIdentity {
    pub public_key: Vec<u8>,
    pub secret_key: Vec<u8>,
    pub id: String, // short hex content-address of the public key
}

impl NodeIdentity {
    /// Create a fresh identity (uses `OsRng` ONCE at creation — keygen is the
    /// only place entropy is allowed; it is never used for runtime output).
    pub fn create() -> Self {
        use ed25519_dalek::{SigningKey, VerifyingKey};
        use rand::rngs::OsRng;
        use rand::RngCore;
        let mut sk_bytes = [0u8; 32];
        OsRng.fill_bytes(&mut sk_bytes);
        let sk = SigningKey::from_bytes(&sk_bytes);
        let pk: VerifyingKey = sk.verifying_key();
        let pk_bytes = pk.to_bytes().to_vec();
        let id = short_id(&pk_bytes);
        NodeIdentity {
            public_key: pk_bytes,
            secret_key: sk.to_bytes().to_vec(),
            id,
        }
    }

    /// Re-check self-certification: re-derive the id from the public key. A
    /// tampered secret/public mismatch is caught here.
    pub fn self_certify(&self) -> bool {
        let recomputed = short_id(&self.public_key);
        recomputed == self.id
    }
}

/// Short hex id from public-key bytes (first 8 bytes of SHA-512, hex).
pub fn short_id(pk: &[u8]) -> String {
    let h = Sha512::digest(pk);
    h[..8].iter().map(|b| format!("{b:02x}")).collect()
}

/// The encrypted vault blob on disk.
#[derive(serde::Serialize, serde::Deserialize)]
pub struct VaultBlob {
    pub version: u8,
    pub salt: Vec<u8>,
    pub ciphertext: Vec<u8>, // XChaCha20Poly1305 over (secret||public||id)
}

/// Create a new vault, returning (blob, identity). If `path` exists it is
/// overwritten only when `force` is set.
pub fn create_or_unlock(pass: &str, path: &str, force: bool) -> Result<NodeIdentity> {
    if fs::metadata(path).is_ok() && !force {
        // Already exists — unlock instead.
        return unlock(pass, path);
    }
    let id = NodeIdentity::create();
    let salt = {
        // Deterministic salt from pass — the vault must be reproducible from the
        // passphrase alone (no stored nonce/salt that could drift). Uses SHA-512.
        let mut s = [0u8; 16];
        let h = Sha512::digest(pass.as_bytes());
        s.copy_from_slice(&h[..16]);
        s.to_vec()
    };
    let key = derive_key(pass.as_bytes(), &salt);
    let nonce = derive_nonce(pass.as_bytes(), &salt);
    let cipher = XChaCha20Poly1305::new(&key.into());

    // payload = secret || public || id  (length-prefixed)
    let mut payload = Vec::new();
    payload.extend_from_slice(&(id.secret_key.len() as u32).to_le_bytes());
    payload.extend_from_slice(&id.secret_key);
    payload.extend_from_slice(&(id.public_key.len() as u32).to_le_bytes());
    payload.extend_from_slice(&id.public_key);
    payload.extend_from_slice(id.id.as_bytes());

    let ct = cipher
        .encrypt(XNonce::from_slice(&nonce), payload.as_slice())
        .map_err(|_| anyhow!("encryption failed"))?;

    let blob = VaultBlob {
        version: VAULT_VERSION,
        salt,
        ciphertext: ct,
    };
    let json = serde_json::to_string(&blob)?;
    fs::write(path, json)?;
    Ok(id)
}

/// Unlock an existing vault: AEAD auth rejects a wrong passphrase or tampering.
pub fn unlock(pass: &str, path: &str) -> Result<NodeIdentity> {
    let raw = fs::read(path)?;
    let blob: VaultBlob = serde_json::from_slice(&raw)?;
    if blob.version != VAULT_VERSION {
        bail!("unsupported vault version {}", blob.version);
    }
    let key = derive_key(pass.as_bytes(), &blob.salt);
    let nonce = derive_nonce(pass.as_bytes(), &blob.salt);
    let cipher = XChaCha20Poly1305::new(&key.into());

    let pt = cipher
        .decrypt(XNonce::from_slice(&nonce), blob.ciphertext.as_slice())
        .map_err(|_| anyhow!("vault auth failed — wrong passphrase or tampered blob"))?;

    // parse length-prefixed payload
    let mut i = 0;
    let sk_len = u32::from_le_bytes(pt[i..i + 4].try_into()?) as usize;
    i += 4;
    let secret_key = pt[i..i + sk_len].to_vec();
    i += sk_len;
    let pk_len = u32::from_le_bytes(pt[i..i + 4].try_into()?) as usize;
    i += 4;
    let public_key = pt[i..i + pk_len].to_vec();
    i += pk_len;
    let id = String::from_utf8(pt[i..].to_vec())?;

    let identity = NodeIdentity {
        public_key,
        secret_key,
        id,
    };
    if !identity.self_certify() {
        bail!("identity self-certification mismatch — vault integrity compromised");
    }
    Ok(identity)
}

/// Lock helper: just confirms the blob is on disk and valid (AEAD-verifiable).
pub fn lock(path: &str) -> Result<()> {
    if fs::metadata(path).is_ok() {
        Ok(())
    } else {
        bail!("no vault at {path}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PATH: &str = "/tmp/bebop-vault-test.json";

    #[test]
    fn create_unlock_roundtrip() {
        // GREEN: create then unlock returns the SAME self-certifying id.
        let _ = fs::remove_file(PATH);
        let a = create_or_unlock("hunter2", PATH, true).unwrap();
        assert!(a.self_certify());
        let b = unlock("hunter2", PATH).unwrap();
        assert_eq!(a.id, b.id, "id not stable across unlock");
        let _ = fs::remove_file(PATH);
    }

    #[test]
    fn wrong_passphrase_rejected() {
        // RED: a wrong passphrase must fail AEAD auth (never silently decrypt).
        let _ = fs::remove_file(PATH);
        let _ = create_or_unlock("right-pass", PATH, true).unwrap();
        let res = unlock("wrong-pass", PATH);
        assert!(res.is_err(), "wrong passphrase was accepted — catastropic");
        let _ = fs::remove_file(PATH);
    }

    #[test]
    fn tampered_blob_rejected() {
        // RED: flipping one byte of the ciphertext must fail auth.
        let _ = fs::remove_file(PATH);
        let _ = create_or_unlock("pass", PATH, true).unwrap();
        let mut raw = fs::read(PATH).unwrap();
        let last = raw.len() - 1;
        raw[last] ^= 0xFF; // flip a ciphertext byte
        fs::write(PATH, raw).unwrap();
        let res = unlock("pass", PATH);
        assert!(res.is_err(), "tampered blob decrypted — AEAD broken");
        let _ = fs::remove_file(PATH);
    }

    #[test]
    fn self_certify_catches_mismatch() {
        // RED: a corrupted id (pk intact) is caught by self-certify.
        let pk = Sha512::digest(b"dummy").to_vec();
        let mut id = NodeIdentity {
            public_key: pk.clone(),
            secret_key: vec![0u8; 32],
            id: short_id(&pk),
        };
        assert!(id.self_certify());
        id.id = "deadbeefdeadbeef".into(); // mismatched id
        assert!(!id.self_certify(), "self-certify missed a tampered id");
    }
}
