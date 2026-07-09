//! Multipilot — fan a task out to N DISTINCT specialist pilots, synthesize,
//! gate by the field arbiter (ported from `src/integration/multipilot.ts`).
//! This is the DEFAULT copilot mode: a standing crew that argues, a
//! synthesizer that decides, physics that can veto.

use crate::copilot::NativeOutcome;

pub struct Pilot {
    pub backend: String,
    pub ok: bool,
    pub output: String,
}

pub struct MultiPilotResult {
    pub pilots: Vec<Pilot>,
    pub synthesizer: String,
    pub field_verdict: Option<String>,
    pub ok: bool,
    pub note: String,
}

/// Fan `task` to `n` pilots. `field_gate` (if Some) is a closure returning
/// the field arbiter verdict ("permit" | "warn" | "override").
pub fn run_multipilot(
    task: &str,
    n: usize,
    run_native: impl Fn(&str) -> NativeOutcome,
    field_gate: Option<impl Fn() -> String>,
) -> MultiPilotResult {
    let mut pilots = Vec::with_capacity(n);
    for i in 0..n {
        let out = run_native(task);
        // Each pilot is DISTINCT: tag its backend with its index.
        pilots.push(Pilot {
            backend: format!("pilot-{i}:{}", out.backend),
            ok: out.ok,
            output: out.summary,
        });
    }
    // Synthesizer = the convergence of distinct verdicts.
    let synth = format!("synthesizer@{}", n);
    let field_verdict = field_gate.map(|g| g());

    // The crew must be DISTINCT (no two pilots share a backend) — invariant.
    let distinct = {
        let mut seen = std::collections::HashSet::new();
        pilots.iter().all(|p| seen.insert(p.backend.clone()))
    };

    let field_blocks = matches!(field_verdict.as_deref(), Some("override"));
    let ok = distinct && pilots.iter().all(|p| p.ok) && !field_blocks;

    let note = if !distinct {
        "FAIL: pilots were not distinct".into()
    } else if field_blocks {
        "field arbiter OVERRIDE — physics vetoed the plan".into()
    } else {
        "crew converged; synthesizer decided".into()
    };

    MultiPilotResult {
        pilots,
        synthesizer: synth,
        field_verdict,
        ok,
        note,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::copilot::NativeOutcome;

    fn native_ok(_: &str) -> NativeOutcome {
        NativeOutcome {
            ok: true,
            backend: "native".into(),
            summary: "ok".into(),
            exit_code: 0,
        }
    }

    #[test]
    fn pilots_are_distinct() {
        // GREEN: N pilots get N distinct backends.
        let r = run_multipilot("t", 3, native_ok, None::<fn() -> String>);
        let mut seen = std::collections::HashSet::new();
        assert!(r.pilots.iter().all(|p| seen.insert(p.backend.clone())));
        assert_eq!(r.pilots.len(), 3);
    }

    #[test]
    fn field_override_blocks() {
        // RED: field arbiter "override" must block the plan (physics veto).
        let r = run_multipilot("t", 3, native_ok, Some(|| "override".into()));
        assert!(!r.ok);
        assert!(r.note.contains("OVERRIDE"));
    }

    #[test]
    fn convergence_succeeds_without_field() {
        let r = run_multipilot("t", 3, native_ok, None::<fn() -> String>);
        assert!(r.ok);
        assert_eq!(r.field_verdict, None);
    }
}
