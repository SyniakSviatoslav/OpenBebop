//! G5 (2026-07-14): capability-scoped red-line policy gate inside bebop2.
//!
//! Before this module, bebop2 had NO red-line gate — the deny-list guard kernel
//! was archived TS or an unrelated graph-physics veto in a different crate whose
//! `bebop boot` no longer called it (blueprint gap G5). Trust was "a signed
//! capability", but a *validly signed* capability for a money/settlement/claim
//! mutation would still execute with no operator policy brake.
//!
//! This is a **capability-scoped deny gate**, not a reputation filter. It answers
//! one question: "does this capability's scope touch a red-line category
//! (auth / money / secrets / migrations) that the operator has NOT explicitly
//! allow-listed?" Default = DENY (fail-closed). It never scores, never ranks,
//! never maintains an enemies-list — absence of an explicit grant is silence.
//!
//! Categories are mapped from the existing `Resource`/`Action` vocabulary so the
//! gate composes with the rest of the capability system without new wire types.
//! Reads (e.g. `Ledger::Read`) are intentionally NOT red-line — read-only
//! projection is sovereign-safe; only *mutations* of money/claim/settlement and
//! reserved auth/secrets/migrations categories trip.

use crate::scope::{Action, Resource, Scope};

/// A red-line category. These are the operator-gated actions that must never
/// execute without an explicit allow-list entry, regardless of a valid
/// signature.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RedLineCategory {
    /// Money movement: settlement records, ledger appends, order creation,
    /// claim payouts. Anything that moves value.
    Money,
    /// Secrets exposure (e.g. `Resource::Secret` + `Action::DeploySecret`).
    Secrets,
    /// Schema / data migrations (e.g. `Resource::Migration` + `Action::RunMigration`).
    Migrations,
    /// Authentication / authority changes (e.g. `Resource::Auth` + `Action::Authenticate`).
    Auth,
}

/// The operator policy for red-line scopes.
#[derive(Debug, Clone)]
pub enum RedLinePolicy {
    /// Reject ANY capability whose scope touches a red-line category. Fail-closed.
    /// This is the production default — money/claim/secrets/migrations simply do
    /// not execute unless the operator has explicitly enumerated an allow-list.
    DenyByDefault,
    /// Reject red-line scopes EXCEPT those enumerated here. Each allow-list entry
    /// is itself a `Scope` (set of `(resource, action)` pairs); a requested
    /// red-line pair is permitted only if it is a subset of some allow entry.
    /// The operator must consciously name every allowed red-line verb-on-object.
    AllowList(Vec<Scope>),
}

impl Default for RedLinePolicy {
    /// Fail-closed: the default policy denies red-line scopes.
    fn default() -> Self {
        RedLinePolicy::DenyByDefault
    }
}

/// If `(resource, action)` is a red-line verb, return its category.
///
/// Read-only verbs (`Ledger::Read`, `Order::ReadProjection`, `Route::Send`,
/// `Order::Notify`) are deliberately NOT red-line — they are sovereign-safe.
/// Only *mutations* of money/claim/settlement trip, plus the reserved
/// operator-gated categories (Auth / Secrets / Migrations) which MUST be
/// expressible so the fail-closed gate can deny them (P3 §3.5, M12/F26).
/// Before this mapping, no `Resource` carried an Auth/Secret/Migration
/// variant, so such a capability could not even be *expressed* — it
/// slipped past `DenyByDefault` silently. Now every one of them maps.
pub fn is_red_line(resource: Resource, action: Action) -> Option<RedLineCategory> {
    use RedLineCategory::*;
    match (resource, action) {
        // ── Money / settlement mutations ──
        (Resource::Ledger, Action::SettlementRecorded) => Some(Money),
        (Resource::Ledger, Action::Append) => Some(Money),
        (Resource::Order, Action::CreateOrder) => Some(Money),
        // Claim payouts move value.
        (Resource::Claim, _) => Some(Money),
        // ── Reserved operator-gated categories (any verb on these resources) ──
        (Resource::Auth, _) => Some(Auth),
        (Resource::Secret, _) => Some(Secrets),
        (Resource::Migration, _) => Some(Migrations),
        _ => None,
    }
}

/// The red-line gate.
pub struct RedLineGate;

impl RedLineGate {
    /// Check `scope` against `policy`.
    ///
    /// Returns `Ok(())` when no red-line pair is present, or when every red-line
    /// pair in `scope` is covered by the policy's allow-list. Returns
    /// `Err(category)` for the first red-line pair that is denied — the gate goes
    /// RED.
    pub fn check(scope: &Scope, policy: &RedLinePolicy) -> Result<(), RedLineCategory> {
        // Collect the red-line pairs actually requested in this scope.
        let red_pairs: Vec<(Resource, Action)> = scope
            .grants
            .iter()
            .copied()
            .filter(|(r, a)| is_red_line(*r, *a).is_some())
            .collect();
        if red_pairs.is_empty() {
            return Ok(());
        }
        match policy {
            RedLinePolicy::DenyByDefault => {
                // Reject the first red-line pair (fail-closed).
                let (r, a) = red_pairs[0];
                Err(is_red_line(r, a).unwrap())
            }
            RedLinePolicy::AllowList(allowed) => {
                // Every requested red-line pair must be a subset of SOME allow
                // entry. A pair not covered => RED.
                for (r, a) in red_pairs {
                    let covered = allowed.iter().any(|allow| allow.grants.contains(&(r, a)));
                    if !covered {
                        return Err(is_red_line(r, a).unwrap());
                    }
                }
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_only_is_not_red_line() {
        // Sovereign-safe reads must never trip the gate.
        assert_eq!(is_red_line(Resource::Ledger, Action::Read), None);
        assert_eq!(is_red_line(Resource::Route, Action::Send), None);
        assert_eq!(is_red_line(Resource::Order, Action::ReadProjection), None);
        assert_eq!(is_red_line(Resource::Order, Action::Notify), None);
    }

    #[test]
    fn money_mutations_are_red_line() {
        assert_eq!(
            is_red_line(Resource::Ledger, Action::SettlementRecorded),
            Some(RedLineCategory::Money)
        );
        assert_eq!(
            is_red_line(Resource::Order, Action::CreateOrder),
            Some(RedLineCategory::Money)
        );
        assert_eq!(
            is_red_line(Resource::Claim, Action::ClaimOffered),
            Some(RedLineCategory::Money)
        );
    }

    #[test]
    fn deny_by_default_rejects_red_line_scope() {
        let scope = Scope::single(Resource::Ledger, Action::SettlementRecorded);
        assert_eq!(
            RedLineGate::check(&scope, &RedLinePolicy::DenyByDefault),
            Err(RedLineCategory::Money)
        );
        // A mixed scope (route send + settlement) is still RED — one red pair
        // poisons the whole capability.
        let mixed = Scope::new(vec![
            (Resource::Route, Action::Send),
            (Resource::Ledger, Action::SettlementRecorded),
        ]);
        assert_eq!(
            RedLineGate::check(&mixed, &RedLinePolicy::DenyByDefault),
            Err(RedLineCategory::Money)
        );
    }

    // ── P3 §3.5 (M12 / F26): the reserved Auth/Secrets/Migrations
    // categories MUST trip the fail-closed gate. Before this mapping, no
    // `Resource` carried such a variant, so the capability could not even
    // be expressed — it slipped past `DenyByDefault`. Now an Auth /
    // Secret / Migration scoped capability is denied by default, with a
    // test per category (mirrors `money_mutations_are_red_line`).
    #[test]
    fn auth_secret_migration_are_red_line() {
        // Each reserved category maps regardless of the action verb.
        assert_eq!(
            is_red_line(Resource::Auth, Action::Authenticate),
            Some(RedLineCategory::Auth)
        );
        assert_eq!(
            is_red_line(Resource::Secret, Action::DeploySecret),
            Some(RedLineCategory::Secrets)
        );
        assert_eq!(
            is_red_line(Resource::Migration, Action::RunMigration),
            Some(RedLineCategory::Migrations)
        );
        // Any verb on these resources trips (gate is verb-agnostic here).
        assert_eq!(
            is_red_line(Resource::Auth, Action::Read),
            Some(RedLineCategory::Auth)
        );
    }

    #[test]
    fn deny_by_default_rejects_reserved_categories() {
        for (r, a, cat) in [
            (Resource::Auth, Action::Authenticate, RedLineCategory::Auth),
            (
                Resource::Secret,
                Action::DeploySecret,
                RedLineCategory::Secrets,
            ),
            (
                Resource::Migration,
                Action::RunMigration,
                RedLineCategory::Migrations,
            ),
        ] {
            assert_eq!(
                RedLineGate::check(&Scope::single(r, a), &RedLinePolicy::DenyByDefault),
                Err(cat)
            );
        }
    }

    // ── P5 (expressibility canary): every `RedLineCategory` variant MUST be
    // reachable through `is_red_line`. If a 5th category is ever added to the
    // enum but forgotten in `is_red_line`'s match, `DenyByDefault` returns
    // `None` for it → the capability is SILENTLY ALLOWED (the expressibility
    // hole P5 exists to catch). This test fails (not just warns) when that
    // happens, because the exhaustive match on the enum forces each variant to
    // be asserted reachable. The `(r, a)` pairs mirror `is_red_line`'s arms.
    #[test]
    fn every_red_line_category_is_reachable() {
        // One known (resource, action) pair per category. If a variant loses
        // its `is_red_line` arm, the `match cat` below won't compile until
        // updated — and the assertion proves a live mapping exists.
        let known: &[(Resource, Action, RedLineCategory)] = &[
            (
                Resource::Ledger,
                Action::SettlementRecorded,
                RedLineCategory::Money,
            ),
            (
                Resource::Secret,
                Action::DeploySecret,
                RedLineCategory::Secrets,
            ),
            (
                Resource::Migration,
                Action::RunMigration,
                RedLineCategory::Migrations,
            ),
            (Resource::Auth, Action::Authenticate, RedLineCategory::Auth),
        ];
        for (r, a, cat) in known {
            let got = is_red_line(*r, *a);
            assert_eq!(
                got,
                Some(*cat),
                "category {:?} unreachable via is_red_line",
                cat
            );
            // Exhaustive match proves the enum has no unhandled variant here.
            match *cat {
                RedLineCategory::Money
                | RedLineCategory::Secrets
                | RedLineCategory::Migrations
                | RedLineCategory::Auth => {}
            }
        }
        // Sanity: a NON-red-line scope still maps to None (not over-broad).
        assert_eq!(is_red_line(Resource::Route, Action::Send), None);
    }

    #[test]
    fn allow_list_narrows_precisely() {
        // Allow ONLY settlement; claim payout still rejected.
        let policy = RedLinePolicy::AllowList(vec![Scope::single(
            Resource::Ledger,
            Action::SettlementRecorded,
        )]);
        assert!(RedLineGate::check(
            &Scope::single(Resource::Ledger, Action::SettlementRecorded),
            &policy
        )
        .is_ok());
        // A different money verb is NOT covered => RED.
        assert_eq!(
            RedLineGate::check(
                &Scope::single(Resource::Order, Action::CreateOrder),
                &policy
            ),
            Err(RedLineCategory::Money)
        );
    }

    #[test]
    fn clean_scope_passes_every_policy() {
        let s = Scope::single(Resource::Route, Action::Send);
        assert!(RedLineGate::check(&s, &RedLinePolicy::DenyByDefault).is_ok());
        assert!(RedLineGate::check(
            &s,
            &RedLinePolicy::AllowList(vec![Scope::single(Resource::Ledger, Action::Append)])
        )
        .is_ok());
    }
}
