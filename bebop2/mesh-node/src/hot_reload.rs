//! P10 (§8.2 DECART) — the hot-reload watcher: a stat-`mtime` poll loop.
//!
//! DECART decision: hot-reload uses `std::fs::metadata(..).modified()` polling —
//! ZERO new dependency (no `notify`/inotify crate). The watcher records the
//! last-seen mtime; [`PolicyWatcher::changed`] returns `true` exactly once per
//! observed change. The async runtime calls it on an interval and, on `true`,
//! reloads + `apply_revision`s the policy (floor-gated). A read error (file
//! transiently gone mid-edit) is NOT a change and NOT a failure — the last-good
//! policy stays live (fail-safe under a racing editor).
//!
//! CI GUARD: NO-COURIER-SCORING — the watcher polls an mtime, no score.

use std::time::SystemTime;

/// Tracks a file's last-seen modification time to detect edits by polling.
#[derive(Debug)]
pub struct PolicyWatcher {
    path: String,
    last_mtime: Option<SystemTime>,
}

impl PolicyWatcher {
    /// Build a watcher for `path`, seeding the current mtime so the first
    /// `changed()` after boot does not spuriously fire on an unmodified file.
    pub fn new(path: &str) -> Self {
        let last_mtime = current_mtime(path);
        PolicyWatcher {
            path: path.to_string(),
            last_mtime,
        }
    }

    /// The watched path.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Poll once. Returns `true` iff the file's mtime advanced since the last
    /// observed value (i.e. the operator edited it). Updates the stored mtime so
    /// the next poll only fires on the NEXT edit. A missing/unreadable file is
    /// treated as "no change" (last-good stays live).
    pub fn changed(&mut self) -> bool {
        match current_mtime(&self.path) {
            Some(m) => {
                let advanced = match self.last_mtime {
                    Some(prev) => m > prev,
                    None => true, // file appeared where there was none
                };
                if advanced {
                    self.last_mtime = Some(m);
                }
                advanced
            }
            None => false, // gone/unreadable mid-edit => not a change
        }
    }
}

fn current_mtime(path: &str) -> Option<SystemTime> {
    std::fs::metadata(path).ok()?.modified().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    // ── §6.8 support: mtime poll detects an edit without a restart ──
    #[test]
    fn watcher_detects_edit_via_mtime() {
        let dir = std::env::temp_dir();
        let p = dir.join(format!("p10-watch-{}.txt", std::process::id()));
        std::fs::write(&p, "v1").unwrap();
        let mut w = PolicyWatcher::new(p.to_str().unwrap());
        // No edit yet => no change.
        assert!(!w.changed());
        // Sleep past filesystem mtime granularity, then edit.
        std::thread::sleep(Duration::from_millis(1100));
        std::fs::write(&p, "v2").unwrap();
        assert!(w.changed(), "edit must be detected");
        // A second poll with no further edit => no change.
        assert!(!w.changed());
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn watcher_missing_file_is_no_change() {
        let mut w = PolicyWatcher::new("/nonexistent/p10-none.txt");
        assert!(!w.changed());
    }
}
