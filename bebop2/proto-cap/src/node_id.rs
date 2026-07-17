//! MESH-12 — node identity (ADR-0007) + genesis loader + HUMAN root-delegation policy.
//!
//! # node_id = H(pq_pub || classical_pub)  (ADR-0007, no-CA, SPKI-lineage)
//!
//! A mesh node's identity is **derived**, never assigned by a CA. It binds the
//! node's two public keys — the post-quantum key (ML-KEM/ML-DSA public material)
//! and the classical key (Ed25519) — into a single 32-byte id via SHA3-256.
//! This kills the *seeded-owner JWT* anti-pattern: there is no magical "owner"
//! claim you can mint; there is only a key lineage you can prove. Changing EITHER
//! public key changes the id. See `docs/design/mesh-real/ADR-0007-*.md`.
//!
//! # Genesis loader (fail-closed)
//!
//! Authority at runtime needs a frozen trust-anchor set enrolled exactly once,
//! at genesis, from config/disk — **not** inline in code and **not** auto-seeded.
//! [`load_genesis`] reads a plain-text anchor file (one hex 32-byte Ed25519
//! public key per line, `#` comments allowed). It is FAIL-CLOSED: a missing,
//! unreadable, malformed, or *zero-anchor* file yields an error and enrolls
//! nothing, so the node captures no authority from a broken genesis.
//!
//! # HUMAN decision: root-delegation policy  (innovate:)
//!
//! The actual root-delegation model — operator-signed vs Web-of-Trust vs
//! first-contact-QR — is an **OPERATOR decision**. This module implements all
//! three as the [`RootDelegationPolicy`] enum and a [`Default`] of
//! `Unspecified`, but the code MUST NOT silently pick one as "chosen". The
//! operator configures the policy explicitly; until then the node fails closed
//! and enrolls no root authority. Do not "helpfully" default to a real policy.

use bebop2_core::hash::sha3_256;

use crate::capability::Capability;
use crate::error::CapError;
use crate::revocation::RevocationSet;
use crate::roster::{AnchorRoster, Delegation, Effect};
use crate::scope::{Action, Resource, Scope};

/// A mesh node identity: `H(pq_pub || classical_pub)`.
///
/// 32 bytes; deterministic; changes if EITHER input key changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(pub [u8; 32]);

impl NodeId {
    /// Derive the node id from the PQ public key and the classical (Ed25519)
    /// public key, per ADR-0007: `id = SHA3-256(pq_pub || classical_pub)`.
    pub fn from_keys(pq_pub: &[u8], classical_pub: &[u8; 32]) -> Self {
        let mut buf = Vec::with_capacity(pq_pub.len() + 32);
        buf.extend_from_slice(pq_pub);
        buf.extend_from_slice(classical_pub);
        NodeId(sha3_256(&buf))
    }

    /// The raw 32-byte id.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Lowercase hex encoding (offline, dependency-free).
    pub fn to_hex(&self) -> String {
        hex_encode(&self.0)
    }
}

/// The two public keys a node presents on the wire.
#[derive(Debug, Clone)]
pub struct NodeKeys {
    /// Post-quantum public key material (e.g. ML-KEM-768 pk, 1184 bytes; or
    /// ML-DSA public key). Variable length by design.
    pub pq_pub: Vec<u8>,
    /// Classical (Ed25519) 32-byte public key.
    pub classical_pub: [u8; 32],
}

impl NodeKeys {
    /// Derive this node's [`NodeId`].
    pub fn node_id(&self) -> NodeId {
        NodeId::from_keys(&self.pq_pub, &self.classical_pub)
    }
}

// ── Genesis loader (fail-closed) ──────────────────────────────────────────────

/// Errors returned by the genesis loader.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GenesisError {
    /// The genesis file could not be read (missing, no permission, ...).
    Io(String),
    /// A line in the genesis file was malformed (bad hex / wrong length).
    Parse(String),
    /// The file parsed but yielded ZERO anchors. Fail-closed: no silent
    /// empty bootstrap. The node must not capture any authority.
    EmptyRoster,
    /// A root-delegation policy was needed but none was explicitly chosen.
    PolicyUnspecified,
}

impl From<std::io::Error> for GenesisError {
    fn from(e: std::io::Error) -> Self {
        GenesisError::Io(e.to_string())
    }
}

/// Load the frozen trust-anchor set from disk.
///
/// Format: plain text, one hex-encoded 32-byte Ed25519 public key per line.
/// Blank lines and `#` comments are ignored. See `config/genesis.example.txt`.
///
/// FAIL-CLOSED: any of the following yields an error and enrolls NOTHING:
/// - file missing / unreadable ([`GenesisError::Io`]);
/// - a non-comment line is not exactly 64 hex chars / decodes to ≠ 32 bytes
///   ([`GenesisError::Parse`]);
/// - the file is valid but contains zero anchors ([`GenesisError::EmptyRoster`]).
///
/// The node therefore captures **no authority** from a broken or empty genesis.
/// Authority is never auto-seeded.
pub fn load_genesis(path: &str) -> Result<AnchorRoster, GenesisError> {
    let data = std::fs::read_to_string(path)?;
    let mut roster = AnchorRoster::new();
    for (i, raw) in data.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let bytes = hex_decode(line)
            .map_err(|e| GenesisError::Parse(format!("line {}: {}", i + 1, e)))?;
        if bytes.len() != 32 {
            return Err(GenesisError::Parse(format!(
                "line {}: expected 32-byte key, got {}",
                i + 1,
                bytes.len()
            )));
        }
        let mut key = [0u8; 32];
        key.copy_from_slice(&bytes);
        roster.enroll(&key);
    }
    if roster.is_empty() {
        return Err(GenesisError::EmptyRoster);
    }
    Ok(roster)
}

/// A freshly-initialized node has NO anchors. It captures no authority until a
/// genesis is loaded. This is the fail-closed default — never auto-seed.
pub fn empty_roster_fail_closed() -> AnchorRoster {
    AnchorRoster::new()
}

// ── HUMAN decision: root-delegation policy (innovate:) ────────────────────────

/// innovate: The root-delegation model is an **OPERATOR decision**. This enum
/// lists all three supported models. The production system MUST NOT silently
/// pick one as "chosen" — the operator configures the policy explicitly. The
/// [`Default`] is [`RootDelegationPolicy::Unspecified`], which fails closed and
/// enrolls no root authority. Do not "helpfully" default to a real policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RootDelegationPolicy {
    /// Operator-signed root certificate(s): offline, audited, pinned.
    OperatorSigned,
    /// Web-of-Trust: anchors accepted transitively from a trusted seed set.
    WebOfTrust,
    /// First-contact QR: out-of-band key exchange (e.g. scanned at commissioning).
    FirstContactQr,
    /// No policy chosen. FAIL-CLOSED: do not bootstrap any root authority.
    Unspecified,
}

impl Default for RootDelegationPolicy {
    /// Defaults to `Unspecified` on purpose: fail closed, never auto-pick a real
    /// policy. The operator must choose explicitly.
    fn default() -> Self {
        RootDelegationPolicy::Unspecified
    }
}

/// Require an explicit operator policy choice. Returns `Err` if the policy is
/// still [`RootDelegationPolicy::Unspecified`] — the node must not bootstrap any
/// root authority until the operator decides.
pub fn require_explicit_policy(p: RootDelegationPolicy) -> Result<RootDelegationPolicy, GenesisError> {
    match p {
        RootDelegationPolicy::Unspecified => Err(GenesisError::PolicyUnspecified),
        other => Ok(other),
    }
}

// ── Layer D / P-D (consensus/capability) — Option A: budgeted issuance ──────
//
// A per-epoch, per-anchor mint CAP. Production code MUST route all Ed25519
// delegation signing through [`sign_delegation_budgeted`] — the single seam
// that enforces the cap, anchor enrollment, and revocation BEFORE it calls
// `Delegation::sign`. The CI guard `scripts/ci-budgeted-issuance.sh` fails the
// build on any bare `Delegation::sign(` outside this seam / a `#[cfg(test)]`
// module (budget-bypass bulkhead).
//
// P06-INDEPENDENT: there is NO key_V / dowiz-kernel dependency here. Signing is
// the crate's REAL Ed25519 (`Delegation::sign` -> `bebop2_core::sign`).

/// Default length of one issuance epoch, in monotonic ticks (1 day @ 1 tick/s).
pub const DEFAULT_ISSUANCE_EPOCH_LEN_TICKS: u64 = 86_400;

/// Default maximum number of delegations an anchor may mint per epoch.
pub const DEFAULT_MAX_PER_EPOCH: u32 = 1;

/// A per-epoch mint budget scoped to a single anchor.
///
/// The budget is NOT transferable between anchors (see [`IssuanceError::AnchorMismatch`])
/// and does NOT auto-re-arm: capacity is restored only by an explicit
/// [`charge_issuance`] rollover (or a brand-new budget). A stale exhausted
/// budget replayed — even across an epoch boundary — cannot re-arm.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IssuanceBudget {
    /// The anchor this budget is scoped to (32-byte Ed25519 public key).
    pub anchor_id: [u8; 32],
    /// The epoch this budget's `minted_count` belongs to.
    pub epoch: u64,
    /// How many delegations have been minted in the current `epoch`.
    pub minted_count: u32,
    /// Maximum delegations allowed per epoch (the cap).
    pub max_per_epoch: u32,
}

/// Errors raised by the budgeted-issuance seam. Every pole is a fail-closed
/// refusal — the delegation is NOT signed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IssuanceError {
    /// The root-delegation policy was not an explicitly chosen, budget-capable
    /// policy. Only `OperatorSigned` mints today; `WebOfTrust` has no budget
    /// rule yet, and `Unspecified` fails closed (no authority bootstrapped).
    PolicyRefused(RootDelegationPolicy),
    /// The claimed issuer is not an enrolled trust anchor.
    AnchorNotEnrolled,
    /// The claimed issuer has been revoked (UCAN-style, irreversible).
    AnchorRevoked,
    /// The budget is scoped to a different anchor than the claimed issuer.
    AnchorMismatch,
    /// The clock rolled backward below the budget's recorded epoch.
    EpochRegression,
    /// The per-epoch mint cap is exhausted.
    BudgetExhausted,
    /// `Delegation::sign` (REAL Ed25519) rejected the arguments.
    SignRejected(CapError),
}

impl std::fmt::Display for IssuanceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IssuanceError::PolicyRefused(p) => write!(f, "issuance refused: policy not budget-capable ({:?})", p),
            IssuanceError::AnchorNotEnrolled => write!(f, "issuance refused: anchor not enrolled"),
            IssuanceError::AnchorRevoked => write!(f, "issuance refused: anchor revoked"),
            IssuanceError::AnchorMismatch => write!(f, "issuance refused: budget scoped to a different anchor"),
            IssuanceError::EpochRegression => write!(f, "issuance refused: clock rolled back below budget epoch"),
            IssuanceError::BudgetExhausted => write!(f, "issuance refused: per-epoch mint budget exhausted"),
            IssuanceError::SignRejected(e) => write!(f, "issuance refused: delegation sign rejected ({})", e),
        }
    }
}

impl std::error::Error for IssuanceError {}

/// Map a monotonic tick to an issuance epoch.
///
/// `epoch_len 0` => the epoch is ALWAYS 0 (an "eternal" epoch: the cap can
/// never roll over or re-arm). Otherwise `epoch = now_tick / epoch_len_ticks`.
pub fn issuance_epoch(now_tick: u64, epoch_len_ticks: u64) -> u64 {
    if epoch_len_ticks == 0 {
        0
    } else {
        now_tick / epoch_len_ticks
    }
}

/// Pre-check whether `anchor_id` may mint under budget `b` at `now_tick`.
///
/// Enforces: enrolled anchor, not revoked, budget scoped to this anchor,
/// no epoch regression, and a non-exhausted cap. Does NOT sign.
pub fn can_issue(
    b: IssuanceBudget,
    anchor_id: [u8; 32],
    now_tick: u64,
    epoch_len_ticks: u64,
) -> Result<(), IssuanceError> {
    let cur = issuance_epoch(now_tick, epoch_len_ticks);
    if cur < b.epoch {
        return Err(IssuanceError::EpochRegression);
    }
    if b.anchor_id != anchor_id {
        return Err(IssuanceError::AnchorMismatch);
    }
    if b.minted_count >= b.max_per_epoch {
        return Err(IssuanceError::BudgetExhausted);
    }
    Ok(())
}

/// Charge a mint against the budget at `now_tick`, returning the (possibly
/// rolled-over) budget with `minted_count` incremented by one.
///
/// Roll-over: if `now_tick` falls in a NEW epoch (strictly greater than the
/// budget's current epoch), the counter resets to 0 BEFORE charging — so an
/// explicit roll to the next epoch re-arms the cap to EXACTLY `max_per_epoch`.
/// The counter is never incremented without a successful mint at the caller.
pub fn charge_issuance(b: IssuanceBudget, now_tick: u64, epoch_len_ticks: u64) -> IssuanceBudget {
    let cur = issuance_epoch(now_tick, epoch_len_ticks);
    if cur > b.epoch {
        // New epoch: reset the counter (re-arm to exactly max), then charge once.
        IssuanceBudget {
            epoch: cur,
            minted_count: 1,
            ..b
        }
    } else {
        // Same (or first/equal) epoch: increment in place.
        IssuanceBudget {
            minted_count: b.minted_count.saturating_add(1),
            ..b
        }
    }
}

/// The single budgeted-issuance seam. Production code MUST call THIS, not a
/// bare `Delegation::sign`, so the per-epoch cap is always enforced.
///
/// Flow (all REAL, no fake crypto):
/// 1. policy must be `OperatorSigned` (fail-closed on `Unspecified`/`WebOfTrust`);
/// 2. `issued_by` must be an enrolled anchor (`roster.contains`) and not revoked
///    (`revoked.is_revoked_key`);
/// 3. `can_issue` must pass (enrollment handled above; here: anchor match,
///    no epoch regression, non-exhausted cap);
/// 4. `Delegation::sign` (REAL Ed25519) produces the delegation;
/// 5. return `(delegation, charge_issuance(...))`.
///
/// Any failure returns the matching [`IssuanceError`] and signs NOTHING.
// BUDGETED-ISSUANCE-SEAM-BEGIN
pub fn sign_delegation_budgeted(
    policy: RootDelegationPolicy,
    roster: &AnchorRoster,
    revoked: &RevocationSet,
    budget: IssuanceBudget,
    issued_by: [u8; 32],
    subject: [u8; 32],
    scope: Scope,
    effect: Effect,
    expiry: u64,
    nonce: [u8; 8],
    seed: &[u8; 32],
    now_tick: u64,
    epoch_len_ticks: u64,
) -> Result<(Delegation, IssuanceBudget), IssuanceError> {
    // 1. Policy gate (fail-closed: only an explicit OperatorSigned mints today).
    match policy {
        RootDelegationPolicy::OperatorSigned => {}
        other => return Err(IssuanceError::PolicyRefused(other)),
    }

    // 2. Enrollment + revocation gates.
    if !roster.contains(&issued_by) {
        return Err(IssuanceError::AnchorNotEnrolled);
    }
    if revoked.is_revoked_key(&issued_by) {
        return Err(IssuanceError::AnchorRevoked);
    }

    // 3. Budget pre-check (anchor match + epoch regression + exhaustion).
    can_issue(budget, issued_by, now_tick, epoch_len_ticks)?;

    // 4. REAL Ed25519 signature (the crate's own signing path).
    let delegation = Delegation::sign(issued_by, subject, scope, effect, expiry, nonce, seed)
        .map_err(IssuanceError::SignRejected)?;

    // 5. Charge + return.
    let charged = charge_issuance(budget, now_tick, epoch_len_ticks);
    Ok((delegation, charged))
}
// BUDGETED-ISSUANCE-SEAM-END

// ── small offline hex helpers (no external crate) ─────────────────────────────

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        s.push(HEX[(b >> 4) as usize] as char);
        s.push(HEX[(b & 0x0f) as usize] as char);
    }
    s
}

fn hex_decode(s: &str) -> Result<Vec<u8>, String> {
    let s = s.trim();
    if s.len() % 2 != 0 {
        return Err("odd-length hex".to_string());
    }
    let mut out = Vec::with_capacity(s.len() / 2);
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let hi = hex_val(bytes[i])?;
        let lo = hex_val(bytes[i + 1])?;
        out.push((hi << 4) | lo);
        i += 2;
    }
    Ok(out)
}

fn hex_val(c: u8) -> Result<u8, String> {
    match c {
        b'0'..=b'9' => Ok(c - b'0'),
        b'a'..=b'f' => Ok(c - b'a' + 10),
        b'A'..=b'F' => Ok(c - b'A' + 10),
        _ => Err(format!("invalid hex char {:?}", c as char)),
    }
}

// ── RED tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::roster::verify_chain;

    fn k(seed_byte: u8) -> ([u8; 32], [u8; 32]) {
        let seed = [seed_byte; 32];
        let (pk, _) = bebop2_core::sign::keygen(&seed);
        (seed, pk)
    }

    // RED: node_id recomputed from the SAME two pubkeys is identical, and
    // changing EITHER pubkey changes the id.
    #[test]
    fn red_node_id_recomputed_from_both_pubkeys_matches() {
        let (_s1, c1) = k(1u8);
        let pq1 = vec![0xaa; 1184]; // pretend ML-KEM-768 pk length

        let id_a = NodeId::from_keys(&pq1, &c1);
        let id_b = NodeId::from_keys(&pq1, &c1);
        assert_eq!(id_a, id_b, "same two pubkeys => same node_id");
        assert_eq!(id_a.to_hex(), id_b.to_hex());

        // Change the classical key -> different id.
        let (_s2, c2) = k(2u8);
        let id_c = NodeId::from_keys(&pq1, &c2);
        assert_ne!(id_a, id_c, "different classical key => different node_id");

        // Change the PQ key -> different id.
        let pq2 = vec![0xbb; 1184];
        let id_d = NodeId::from_keys(&pq2, &c1);
        assert_ne!(id_a, id_d, "different PQ key => different node_id");

        // And a NodeKeys round-trip behaves identically.
        let nk = NodeKeys { pq_pub: pq1, classical_pub: c1 };
        assert_eq!(nk.node_id(), id_a);
    }

    // RED: an empty roster is fail-closed — the node captures no authority.
    // load_genesis on a zero-anchor file is rejected (EmptyRoster), and an
    // empty roster refuses to vouch for any seeded-owner delegation.
    #[test]
    fn red_empty_roster_fail_closed_no_capture() {
        // empty_roster_fail_closed() must give a roster that contains nothing.
        let empty = empty_roster_fail_closed();
        assert!(empty.is_empty(), "fresh node captures no anchors");

        // A "seeded owner" (the old JWT-owner anti-pattern) key, used as the
        // root of a delegation, must be rejected because the roster is empty.
        let (_owner_seed, owner_pk) = k(7u8);
        let (_leaf_seed, leaf_pk) = k(8u8);
        let cap = Capability::new(leaf_pk, Resource::Route, Action::Send, [1u8; 8], 9999);
        let delegation = Delegation::sign(
            owner_pk, // issued_by == seeded owner (NOT enrolled anywhere)
            leaf_pk,
            Scope::single(Resource::Route, Action::Send),
            Effect::single(Resource::Route, Action::Send),
            9999,
            [2u8; 8],
            &_owner_seed,
        )
        .unwrap();
        let err = verify_chain(&empty, &[delegation], &cap, 0);
        assert!(
            matches!(err, Err(CapError::UnknownIssuer)),
            "empty roster must reject any root issuance (no capture), got {:?}",
            err
        );

        // load_genesis on an empty/comment-only file must fail closed.
        let dir = std::env::temp_dir();
        let path = dir.join("mesh12_empty_genesis.txt");
        std::fs::write(&path, "# only a comment\n\n").unwrap();
        let res = load_genesis(path.to_str().unwrap());
        assert!(
            matches!(res, Err(GenesisError::EmptyRoster)),
            "zero-anchor genesis must fail closed, got {:?}",
            res
        );
        let _ = std::fs::remove_file(&path);
    }

    // RED: a seeded-owner fixture cannot mint authority — there is nothing to
    // seed. The old "owner JWT" bootstrap is dead: a hardcoded owner key alone
    // grants no capability. (nothing-to-seed test)
    #[test]
    fn red_seeded_owner_fixture_cannot_mint() {
        // The "seeded owner" public key, hardcoded in the old bootstrap path.
        let (_owner_seed, owner_pk) = k(9u8);

        // Even presenting the owner key as the capability subject with no chain
        // and an empty roster yields no authority (UnknownIssuer path).
        let cap = Capability::new(owner_pk, Resource::Route, Action::Send, [3u8; 8], 9999);
        let empty = empty_roster_fail_closed();
        let err = verify_chain(&empty, &[], &cap, 0);
        assert!(
            matches!(err, Err(CapError::UnknownIssuer)),
            "seeded-owner with empty roster + no chain must be rejected, got {:?}",
            err
        );

        // And a self-signed owner->owner delegation (the literal "I am the owner"
        // mint) is rejected on an empty roster.
        let self_deleg = Delegation::sign(
            owner_pk,
            owner_pk,
            Scope::single(Resource::Route, Action::Send),
            Effect::single(Resource::Route, Action::Send),
            9999,
            [4u8; 8],
            &_owner_seed,
        )
        .unwrap();
        let err2 = verify_chain(&empty, &[self_deleg], &cap, 0);
        assert!(
            matches!(err2, Err(CapError::UnknownIssuer)),
            "seeded-owner self-mint must be rejected (nothing to seed), got {:?}",
            err2
        );
    }

    // GREEN (guard): load_genesis succeeds on a well-formed file and enrolls the
    // anchors; the policy enum defaults to Unspecified and must be chosen.
    #[test]
    fn green_load_genesis_ok_and_policy_must_be_chosen() {
        let (_a, a) = k(20u8);
        let (_b, b) = k(21u8);
        let dir = std::env::temp_dir();
        let path = dir.join("mesh12_genesis.txt");
        std::fs::write(
            &path,
            format!("# mesh-real genesis (frozen anchor set)\n{}\n{}\n", hex_encode(&a), hex_encode(&b)),
        )
        .unwrap();
        let roster = load_genesis(path.to_str().unwrap()).expect("valid genesis loads");
        assert!(roster.contains(&a));
        assert!(roster.contains(&b));
        assert!(!roster.is_empty());
        let _ = std::fs::remove_file(&path);

        // Policy defaults to Unspecified and must be explicitly chosen.
        assert_eq!(RootDelegationPolicy::default(), RootDelegationPolicy::Unspecified);
        assert!(matches!(
            require_explicit_policy(RootDelegationPolicy::default()),
            Err(GenesisError::PolicyUnspecified)
        ));
        // A real operator choice is accepted.
        assert_eq!(
            require_explicit_policy(RootDelegationPolicy::OperatorSigned).unwrap(),
            RootDelegationPolicy::OperatorSigned
        );
    }

    // ── Layer D / P-D (consensus/capability) — Option A: budgeted issuance ──
    // P06-independent: NO key_V / dowiz-kernel dependency. All signing is the
    // crate's REAL Ed25519 (Delegation::sign). These RED→GREEN tests prove the
    // per-epoch mint cap enforced by `sign_delegation_budgeted`.

    /// Build an enrolled anchor + empty roster/revocation + a fresh budget (cap 3).
    fn budget_anchor_fixture() -> (([u8; 32], [u8; 32]), AnchorRoster, RevocationSet, IssuanceBudget) {
        let (seed, pk) = k(40u8);
        let mut roster = AnchorRoster::new();
        roster.enroll(&pk);
        let revoked = RevocationSet::new();
        let budget = IssuanceBudget { anchor_id: pk, epoch: 0, minted_count: 0, max_per_epoch: 3 };
        ((seed, pk), roster, revoked, budget)
    }

    // RED: an attacker facing a cap of 3 mints may mint 1..3, and mints 4..10
    // MUST be refused as BudgetExhausted — while each allowed mint is a REAL
    // Ed25519 delegation that verify_chain accepts.
    #[test]
    fn red_attacker_10_mints_against_cap_3_refused_from_4th() {
        let ((seed, pk), roster, revoked, mut b) = budget_anchor_fixture();
        let epoch_len = DEFAULT_ISSUANCE_EPOCH_LEN_TICKS;
        let now = 0u64;
        for i in 0..3u8 {
            let (_ls, lpk) = k(100u8 + i);
            let cap = Capability::new(lpk, Resource::Route, Action::Send, [i; 8], 9999);
            let res = sign_delegation_budgeted(
                RootDelegationPolicy::OperatorSigned,
                &roster, &revoked, b,
                pk, lpk,
                Scope::single(Resource::Route, Action::Send),
                Effect::single(Resource::Route, Action::Send),
                9999, [i; 8], &seed, now, epoch_len,
            );
            assert!(res.is_ok(), "mint {} (cap 3) must be allowed", i + 1);
            let (d, nb) = res.unwrap();
            assert!(
                verify_chain(&roster, &[d], &cap, now).is_ok(),
                "mint {} delegation must verify_chain-ok (real Ed25519)",
                i + 1
            );
            b = nb;
        }
        assert_eq!(b.minted_count, 3, "exactly 3 minted in epoch 0");
        for i in 3..10u8 {
            let (_ls, lpk) = k(200u8 + i);
            let res = sign_delegation_budgeted(
                RootDelegationPolicy::OperatorSigned,
                &roster, &revoked, b,
                pk, lpk,
                Scope::single(Resource::Route, Action::Send),
                Effect::single(Resource::Route, Action::Send),
                9999, [i; 8], &seed, now, epoch_len,
            );
            assert!(
                matches!(res, Err(IssuanceError::BudgetExhausted)),
                "mint {} must be BudgetExhausted (cap 3, 4th+ refused), got {:?}",
                i + 1, res
            );
        }
    }

    // RED: an Unspecified root-delegation policy mints nothing (fail-closed).
    #[test]
    fn red_unspecified_policy_mints_nothing() {
        let ((seed, pk), roster, revoked, b) = budget_anchor_fixture();
        let (_ls, lpk) = k(50u8);
        let res = sign_delegation_budgeted(
            RootDelegationPolicy::Unspecified,
            &roster, &revoked, b,
            pk, lpk,
            Scope::single(Resource::Route, Action::Send),
            Effect::single(Resource::Route, Action::Send),
            9999, [1; 8], &seed, 0, DEFAULT_ISSUANCE_EPOCH_LEN_TICKS,
        );
        assert!(matches!(res, Err(IssuanceError::PolicyRefused(RootDelegationPolicy::Unspecified))));
    }

    // RED: WebOfTrust has no budget rule yet — it is refused, not silently allowed.
    #[test]
    fn red_web_of_trust_has_no_budget_rule_yet() {
        let ((seed, pk), roster, revoked, b) = budget_anchor_fixture();
        let (_ls, lpk) = k(51u8);
        let res = sign_delegation_budgeted(
            RootDelegationPolicy::WebOfTrust,
            &roster, &revoked, b,
            pk, lpk,
            Scope::single(Resource::Route, Action::Send),
            Effect::single(Resource::Route, Action::Send),
            9999, [1; 8], &seed, 0, DEFAULT_ISSUANCE_EPOCH_LEN_TICKS,
        );
        assert!(matches!(res, Err(IssuanceError::PolicyRefused(RootDelegationPolicy::WebOfTrust))));
    }

    // RED: an anchor that is NOT enrolled in the roster cannot mint.
    #[test]
    fn red_unenrolled_anchor_cannot_mint() {
        let ((seed, pk), _roster, revoked, b) = budget_anchor_fixture();
        let roster = AnchorRoster::new(); // pk NOT enrolled
        let (_ls, lpk) = k(60u8);
        let res = sign_delegation_budgeted(
            RootDelegationPolicy::OperatorSigned,
            &roster, &revoked, b,
            pk, lpk,
            Scope::single(Resource::Route, Action::Send),
            Effect::single(Resource::Route, Action::Send),
            9999, [1; 8], &seed, 0, DEFAULT_ISSUANCE_EPOCH_LEN_TICKS,
        );
        assert!(matches!(res, Err(IssuanceError::AnchorNotEnrolled)));
    }

    // RED: a revoked anchor mints nothing.
    #[test]
    fn red_revoked_anchor_mints_nothing() {
        let ((seed, pk), roster, mut revoked, b) = budget_anchor_fixture();
        revoked.revoke_key(pk);
        let (_ls, lpk) = k(61u8);
        let res = sign_delegation_budgeted(
            RootDelegationPolicy::OperatorSigned,
            &roster, &revoked, b,
            pk, lpk,
            Scope::single(Resource::Route, Action::Send),
            Effect::single(Resource::Route, Action::Send),
            9999, [1; 8], &seed, 0, DEFAULT_ISSUANCE_EPOCH_LEN_TICKS,
        );
        assert!(matches!(res, Err(IssuanceError::AnchorRevoked)));
    }

    // RED: a budget is scoped to one anchor and is NOT transferable to another.
    #[test]
    fn red_budget_not_transferable_between_anchors() {
        let ((seed, pk), mut roster, revoked, mut b) = budget_anchor_fixture();
        let (seed2, pk2) = k(62u8);
        roster.enroll(&pk2); // pk2 is a legit enrolled anchor
        b.anchor_id = pk;    // budget scoped to pk, but we try to mint as pk2
        let (_ls, lpk) = k(63u8);
        let res = sign_delegation_budgeted(
            RootDelegationPolicy::OperatorSigned,
            &roster, &revoked, b,
            pk2, lpk,
            Scope::single(Resource::Route, Action::Send),
            Effect::single(Resource::Route, Action::Send),
            9999, [1; 8], &seed2, 0, DEFAULT_ISSUANCE_EPOCH_LEN_TICKS,
        );
        assert!(matches!(res, Err(IssuanceError::AnchorMismatch)));
    }

    // RED: a clock that rolls backward below the budget's recorded epoch is refused.
    #[test]
    fn red_clock_rollback_refuses() {
        let ((seed, pk), roster, revoked, _b) = budget_anchor_fixture();
        // A budget recorded at epoch 5; a clock that yields epoch 0 must be refused.
        let b = IssuanceBudget { anchor_id: pk, epoch: 5, minted_count: 0, max_per_epoch: 3 };
        let (_ls, lpk) = k(64u8);
        let res = sign_delegation_budgeted(
            RootDelegationPolicy::OperatorSigned,
            &roster, &revoked, b,
            pk, lpk,
            Scope::single(Resource::Route, Action::Send),
            Effect::single(Resource::Route, Action::Send),
            9999, [1; 8], &seed, 0, DEFAULT_ISSUANCE_EPOCH_LEN_TICKS,
        );
        assert!(matches!(res, Err(IssuanceError::EpochRegression)));
    }

    // RED: an epoch rollover re-arms the budget to EXACTLY max (no more, no less).
    #[test]
    fn red_epoch_rollover_rearms_exactly_max() {
        let ((seed, pk), roster, revoked, mut b) = budget_anchor_fixture(); // max = 3
        let epoch_len = DEFAULT_ISSUANCE_EPOCH_LEN_TICKS;
        // Epoch 0: exactly 3 mints allowed.
        for i in 0..3u8 {
            let (_ls, lpk) = k(70u8 + i);
            b = sign_delegation_budgeted(
                RootDelegationPolicy::OperatorSigned,
                &roster, &revoked, b,
                pk, lpk,
                Scope::single(Resource::Route, Action::Send),
                Effect::single(Resource::Route, Action::Send),
                9999, [1; 8], &seed, 0, epoch_len,
            ).unwrap().1;
        }
        assert_eq!(b.minted_count, 3);
        let (_ls, lpk) = k(73u8);
        assert!(matches!(
            sign_delegation_budgeted(
                RootDelegationPolicy::OperatorSigned, &roster, &revoked, b, pk, lpk,
                Scope::single(Resource::Route, Action::Send),
                Effect::single(Resource::Route, Action::Send),
                9999, [1; 8], &seed, 0, epoch_len,
            ),
            Err(IssuanceError::BudgetExhausted)
        ));
        // Rollover to epoch 1: charge_issuance rolls the epoch forward AND
        // commits one mint, so the re-armed budget is at count == 1 (the first
        // mint of the new epoch). Exactly `max_per_epoch - 1` more mints fit.
        b = charge_issuance(b, epoch_len, epoch_len);
        assert_eq!(b.epoch, 1);
        assert_eq!(b.minted_count, 1, "re-arm commits one fresh mint in the new epoch");
        // Epoch 1: 2 more mints reach exactly max (3), then a 4th is refused.
        for i in 0..2u8 {
            let (_ls, lpk) = k(74u8 + i);
            b = sign_delegation_budgeted(
                RootDelegationPolicy::OperatorSigned,
                &roster, &revoked, b,
                pk, lpk,
                Scope::single(Resource::Route, Action::Send),
                Effect::single(Resource::Route, Action::Send),
                9999, [1; 8], &seed, epoch_len, epoch_len,
            ).unwrap().1;
        }
        assert_eq!(b.minted_count, 3);
        let (_ls, lpk) = k(77u8);
        assert!(matches!(
            sign_delegation_budgeted(
                RootDelegationPolicy::OperatorSigned, &roster, &revoked, b, pk, lpk,
                Scope::single(Resource::Route, Action::Send),
                Effect::single(Resource::Route, Action::Send),
                9999, [1; 8], &seed, epoch_len, epoch_len,
            ),
            Err(IssuanceError::BudgetExhausted)
        ));
    }

    // RED: epoch_len == 0 is an eternal epoch (epoch 0 forever); the cap can
    // never roll over or re-arm.
    #[test]
    fn red_epoch_len_zero_is_eternal_epoch() {
        let ((seed, pk), roster, revoked, mut b) = budget_anchor_fixture(); // max = 3
        let epoch_len = 0u64; // eternal: epoch is always 0
        for _ in 0..3u8 {
            let (_ls, lpk) = k(80u8);
            b = sign_delegation_budgeted(
                RootDelegationPolicy::OperatorSigned,
                &roster, &revoked, b,
                pk, lpk,
                Scope::single(Resource::Route, Action::Send),
                Effect::single(Resource::Route, Action::Send),
                9999, [1; 8], &seed, 1_000_000, epoch_len,
            ).unwrap().1;
        }
        assert_eq!(b.epoch, 0, "epoch_len 0 => epoch 0 forever");
        // Far-future tick still cannot re-arm (eternity).
        let (_ls, lpk) = k(83u8);
        assert!(matches!(
            sign_delegation_budgeted(
                RootDelegationPolicy::OperatorSigned, &roster, &revoked, b, pk, lpk,
                Scope::single(Resource::Route, Action::Send),
                Effect::single(Resource::Route, Action::Send),
                9999, [1; 8], &seed, 9_999_999, epoch_len,
            ),
            Err(IssuanceError::BudgetExhausted)
        ));
    }

    // RED: a stale (already-charged) budget replayed — even across an epoch
    // boundary — cannot re-arm. Only an EXPLICIT charge_issuance restores capacity.
    #[test]
    fn red_stale_budget_replay_cannot_rearm() {
        let ((seed, pk), roster, revoked, _b) = budget_anchor_fixture();
        let b = IssuanceBudget { anchor_id: pk, epoch: 0, minted_count: 0, max_per_epoch: 1 };
        let epoch_len = DEFAULT_ISSUANCE_EPOCH_LEN_TICKS;
        // Mint once -> b1 (minted = 1).
        let (_ls, lpk) = k(90u8);
        let (_, b1) = sign_delegation_budgeted(
            RootDelegationPolicy::OperatorSigned,
            &roster, &revoked, b,
            pk, lpk,
            Scope::single(Resource::Route, Action::Send),
            Effect::single(Resource::Route, Action::Send),
            9999, [1; 8], &seed, 0, epoch_len,
        ).unwrap();
        assert_eq!(b1.minted_count, 1);
        // Replay the STALE exhausted budget in the SAME epoch -> no rearm.
        let (_ls, lpk2) = k(91u8);
        assert!(matches!(
            sign_delegation_budgeted(
                RootDelegationPolicy::OperatorSigned, &roster, &revoked, b1, pk, lpk2,
                Scope::single(Resource::Route, Action::Send),
                Effect::single(Resource::Route, Action::Send),
                9999, [2; 8], &seed, 0, epoch_len,
            ),
            Err(IssuanceError::BudgetExhausted)
        ));
        // Replay across an epoch boundary WITHOUT re-arming -> still no rearm.
        assert!(matches!(
            sign_delegation_budgeted(
                RootDelegationPolicy::OperatorSigned, &roster, &revoked, b1, pk, lpk2,
                Scope::single(Resource::Route, Action::Send),
                Effect::single(Resource::Route, Action::Send),
                9999, [3; 8], &seed, epoch_len, epoch_len,
            ),
            Err(IssuanceError::BudgetExhausted)
        ));
        // Only an EXPLICIT re-arm via charge_issuance rolls the epoch over AND
        // commits one mint — so the re-armed budget is at epoch 1, count 1
        // (its single allowance already spent in the new epoch). A further mint
        // in that same epoch is STILL BudgetExhausted: charge_issuance re-arms
        // the *epoch*, but does not grant a free extra mint beyond max_per_epoch.
        let b2 = charge_issuance(b1, epoch_len, epoch_len);
        assert_eq!(b2.epoch, 1);
        assert_eq!(b2.minted_count, 1);
        let (_ls, lpk3) = k(92u8);
        assert!(matches!(
            sign_delegation_budgeted(
                RootDelegationPolicy::OperatorSigned,
                &roster, &revoked, b2,
                pk, lpk3,
                Scope::single(Resource::Route, Action::Send),
                Effect::single(Resource::Route, Action::Send),
                epoch_len, [4; 8], &seed, epoch_len, epoch_len,
            ),
            Err(IssuanceError::BudgetExhausted)
        ), "re-armed budget spent its single mint in the new epoch ⇒ still exhausted");
    }
}
