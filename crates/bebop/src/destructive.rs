//! destructive.rs — configurable destructive/critical change classifier.
//!
//! The operator asked: show destructive/critical changes in the CLI, WITH a
//! configurable definition of what counts as destructive/critical. So the policy
//! is data (patterns + labels), not hardcoded logic. Default policy flags the
//! usual dangerous ops (force-push, reset --hard, rm -rf, red-line areas) as
//! CRITICAL; mass-delete / overwrite as DESTRUCTIVE. User can edit `patterns`.

use crate::changes::ChangeRecord;

/// Severity a change can carry after classification.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Severity {
    None,
    Destructive,
    Critical,
}

/// User-tunable policy. `patterns` are lowercase substrings matched against the
/// change's `target` + `summary`; a matched pattern's `label` decides severity.
/// Convention: label "critical" → Critical, "destructive" → Destructive.
#[derive(Clone, Debug)]
pub struct DestructivePolicy {
    pub patterns: Vec<(String, Severity)>,
}

impl Default for DestructivePolicy {
    /// Operator default: force-push / reset --hard / rm -rf / red-line = Critical;
    /// delete / overwrite / drop = Destructive.
    fn default() -> Self {
        DestructivePolicy {
            patterns: vec![
                ("force-push".into(), Severity::Critical),
                ("--force".into(), Severity::Critical),
                ("reset --hard".into(), Severity::Critical),
                ("rm -rf".into(), Severity::Critical),
                ("drop table".into(), Severity::Critical),
                ("red-line".into(), Severity::Critical),
                ("auth".into(), Severity::Critical),
                ("money".into(), Severity::Critical),
                ("delete".into(), Severity::Destructive),
                ("overwrite".into(), Severity::Destructive),
                ("wipe".into(), Severity::Destructive),
            ],
        }
    }
}

impl DestructivePolicy {
    /// Add/replace a pattern → severity mapping (so the user can tune it).
    pub fn set(&mut self, pattern: &str, sev: Severity) {
        self.patterns.retain(|(p, _)| p != pattern);
        self.patterns.push((pattern.to_string(), sev));
    }
}

/// Classify a change against the policy. Returns the severity and mutates the
/// record's `destructive`/`severity` fields so `changes::render_changes` can
/// promote it. Deterministic: first match wins (patterns are checked in order).
pub fn classify(policy: &DestructivePolicy, rec: &mut ChangeRecord) -> Severity {
    let hay = format!("{} {}", rec.target, rec.summary).to_ascii_lowercase();
    for (pat, sev) in &policy.patterns {
        if hay.contains(&pat.to_ascii_lowercase()) {
            rec.destructive = *sev != Severity::None;
            rec.severity = match sev {
                Severity::None => None,
                Severity::Destructive => Some("destructive".to_string()),
                Severity::Critical => Some("critical".to_string()),
            };
            return sev.clone();
        }
    }
    rec.destructive = false;
    rec.severity = None;
    Severity::None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::changes::ChangeKind;

    #[test]
    fn benign_edit_is_none() {
        // RED: a plain file edit must NOT be flagged.
        let mut r = ChangeRecord::new(ChangeKind::Edit, "crates/bebop/src/foo.rs", "added axis");
        let sev = classify(&DestructivePolicy::default(), &mut r);
        assert_eq!(sev, Severity::None);
        assert!(!r.destructive);
        assert!(r.severity.is_none());
    }

    #[test]
    fn force_push_is_critical() {
        // GREEN: force-push → Critical (operator default).
        let mut r = ChangeRecord::new(ChangeKind::Git, "force-push", "origin");
        let sev = classify(&DestructivePolicy::default(), &mut r);
        assert_eq!(sev, Severity::Critical);
        assert!(r.destructive);
        assert_eq!(r.severity.as_deref(), Some("critical"));
    }

    #[test]
    fn rm_rf_is_critical() {
        let mut r = ChangeRecord::new(ChangeKind::Run, "rm -rf /tmp/build", "clean");
        assert_eq!(classify(&DestructivePolicy::default(), &mut r), Severity::Critical);
    }

    #[test]
    fn delete_is_destructive() {
        // GREEN: plain delete → Destructive (not Critical). Target carries "delete".
        let mut r = ChangeRecord::new(ChangeKind::Delete, "delete old.log", "removed");
        let sev = classify(&DestructivePolicy::default(), &mut r);
        assert_eq!(sev, Severity::Destructive);
        assert_eq!(r.severity.as_deref(), Some("destructive"));
    }

    #[test]
    fn policy_is_user_tunable() {
        // GREEN: user can relax the policy (demote force-push to None).
        let mut pol = DestructivePolicy::default();
        pol.set("force-push", Severity::None);
        let mut r = ChangeRecord::new(ChangeKind::Git, "force-push", "origin");
        assert_eq!(classify(&pol, &mut r), Severity::None);
    }
}
