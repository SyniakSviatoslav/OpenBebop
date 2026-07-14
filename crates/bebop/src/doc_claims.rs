//! doc_claims — Verified-by-Math gate for the Rust core (ported from the
//! `verify-doc-claims.mjs` philosophy). Every doc claim about the core must
//! match live code. This harness is itself a `#[test]` set so the
//! "every test can go RED" invariant survives the TS purge.

#[cfg(test)]
mod tests {
    use crate::launch::{render_launch, Frame};
    use crate::outfit::OUTFIT as O;
    use crate::vault::{create_or_unlock, unlock};
    use crate::OUTFIT;
    use std::fs;

    #[test]
    fn claim_outfit_is_v1() {
        // Claim: the identity contract is version 1.0.0.
        assert_eq!(OUTFIT.version, "1.0.0");
    }

    #[test]
    fn claim_vault_roundtrip_real() {
        // Claim: a created vault unlocks to the SAME self-certifying id.
        let p = "/tmp/bebop-docclaim.json";
        let _ = fs::remove_file(p);
        let a = create_or_unlock("doc-claim-pass", p, true).unwrap();
        let b = unlock("doc-claim-pass", p).unwrap();
        assert_eq!(a.id, b.id);
        let _ = fs::remove_file(p);
    }

    #[test]
    fn claim_launch_is_sun_warm() {
        // Claim (brand law): the launch view carries the sun-warm ship accent.
        let ship = Frame::rgb(O.palette.ship);
        let frames = render_launch(40, 20, 0xC0FFEE, 10);
        let found = frames
            .iter()
            .any(|f| f.cells.iter().any(|&c| (c & 0xFFFFFF) == (ship & 0xFFFFFF)));
        assert!(found, "CLAIM FAILED: launch shows no sun-warm ship");
    }
}
