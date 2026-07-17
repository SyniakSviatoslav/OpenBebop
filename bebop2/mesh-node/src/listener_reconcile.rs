//! P10 (F2) — the dynamic listener open/close reconciler.
//!
//! The `HubPolicy` names which listeners SHOULD be open (deny-by-default: a bind
//! not listed-and-enabled is never opened). The reconciler diffs the *desired*
//! set (from the live policy) against the *actual* set (currently open) and
//! emits the minimal open/close actions to converge — so an operator editing
//! `config/hub-policy.txt` opens/closes ports WITHOUT a restart (§5.1).
//!
//! Pure/deterministic and dependency-free: it computes actions; the async
//! runtime performs the socket open/close. That keeps the decision testable.
//!
//! CI GUARD: NO-COURIER-SCORING — reconciliation compares bind sets, no score.

use std::collections::BTreeSet;

/// A reconcile action to converge the actual listener set to the desired set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ListenerAction {
    /// Open a new listener on this bind address.
    Open(String),
    /// Close an existing listener on this bind address.
    Close(String),
}

/// Compute the minimal set of open/close actions to move `actual` to `desired`.
/// Deterministic order (sorted) so tests and telemetry are stable.
pub fn reconcile(actual: &[String], desired: &[String]) -> Vec<ListenerAction> {
    let actual: BTreeSet<&String> = actual.iter().collect();
    let desired: BTreeSet<&String> = desired.iter().collect();
    let mut actions = Vec::new();
    // Open every desired-but-not-actual (sorted).
    for bind in desired.difference(&actual) {
        actions.push(ListenerAction::Open((*bind).clone()));
    }
    // Close every actual-but-not-desired (sorted).
    for bind in actual.difference(&desired) {
        actions.push(ListenerAction::Close((*bind).clone()));
    }
    actions
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── §6.11 GREEN: adding a listener to policy opens exactly that port ──
    #[test]
    fn reconcile_opens_new_listener() {
        let actions = reconcile(&[], &["0.0.0.0:9443".into()]);
        assert_eq!(actions, vec![ListenerAction::Open("0.0.0.0:9443".into())]);
    }

    // ── §6.11 GREEN: removing a listener closes exactly that port ──
    #[test]
    fn reconcile_closes_dropped_listener() {
        let actions = reconcile(&["0.0.0.0:9443".into()], &[]);
        assert_eq!(actions, vec![ListenerAction::Close("0.0.0.0:9443".into())]);
    }

    #[test]
    fn reconcile_noop_when_converged() {
        let cur = vec!["a:1".to_string(), "b:2".to_string()];
        assert!(reconcile(&cur, &cur).is_empty());
    }

    #[test]
    fn reconcile_mixed_open_and_close() {
        let actual = vec!["keep:1".to_string(), "drop:2".to_string()];
        let desired = vec!["keep:1".to_string(), "add:3".to_string()];
        let actions = reconcile(&actual, &desired);
        assert!(actions.contains(&ListenerAction::Open("add:3".into())));
        assert!(actions.contains(&ListenerAction::Close("drop:2".into())));
        assert_eq!(actions.len(), 2);
    }
}
