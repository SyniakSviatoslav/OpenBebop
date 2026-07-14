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
    /// Secrets exposure (reserved — no `Resource` maps here yet; placeholder so
    /// the gate is future-proof without a wire change).
    Secrets,
    /// Schema / data migrations (reserved — destructive bulk ops).
    Migrations,
    /// Authentication / authority changes (reserved).
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
/// Only *mutations* of money/claim/settlement trip. (The `Secrets`/`Migrations`/
/// `Auth` categories are reserved in [`RedLineCategory`] for future `Resource`
/// variants; this build maps only the money verbs that actually exist.)
pub fn is_red_line(resource: Resource, action: Action) -> Option<RedLineCategory> {
    use RedLineCategory::*;
    match (resource, action) {
        // ── Money / settlement mutations ──
        (Resource::Ledger, Action::SettlementRecorded) => Some(Money),
        (Resource::Ledger, Action::Append) => Some(Money),
        (Resource::Order, Action::CreateOrder) => Some(Money),
        // Claim payouts move value.
        (Resource::Claim, _) => Some(Money),
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
