//! Copilot — the native doer/checker seam (ported from `src/copilot.ts`).
//! The doer produces; a DISTINCT checker verifies in real time. Default on.
//! Field arbiter can veto (see `multipilot` + `field`).

/// A copilot verdict: doer + checker + final ok.
pub struct CopilotResult {
    pub doer: String,
    pub checker: String,
    pub doer_output: String,
    pub verdict: String,
    pub ok: bool,
}

/// Run the copilot seam. `run_native` is the doer (injected by the host).
pub fn run_copilot(
    task: &str,
    enabled: bool,
    run_native: impl Fn(&str) -> NativeOutcome,
) -> CopilotResult {
    let native = run_native(task);
    let checker = if enabled { "kernel::checker" } else { "off" };
    let ok = native.ok && enabled;
    CopilotResult {
        doer: native.backend,
        checker: checker.into(),
        doer_output: native.summary,
        verdict: if ok { "approve" } else { "quarantine" }.into(),
        ok,
    }
}

pub struct NativeOutcome {
    pub ok: bool,
    pub backend: String,
    pub summary: String,
    pub exit_code: i32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copilot_quarantines_when_disabled() {
        // RED+GREEN: with copilot OFF, the verdict must be quarantine (fail-closed).
        let r = run_copilot("do thing", false, |_| NativeOutcome {
            ok: true,
            backend: "native".into(),
            summary: "did".into(),
            exit_code: 0,
        });
        assert!(!r.ok);
        assert_eq!(r.verdict, "quarantine");
    }

    #[test]
    fn copilot_approves_when_doer_ok_and_enabled() {
        let r = run_copilot("do thing", true, |_| NativeOutcome {
            ok: true,
            backend: "native".into(),
            summary: "did".into(),
            exit_code: 0,
        });
        assert!(r.ok);
        assert_eq!(r.verdict, "approve");
    }
}
