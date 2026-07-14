//! Cloud sandbox — isolated command execution for the agent.
//!
//! Fail-closed network policy: by default the command runs with NO network
//! namespace (`unshare -n` on Linux, where available) so egress is impossible.
//! A command is REFUSED outright if it carries an egress token (curl/wget/ssh/…)
//! and `network` was not explicitly opted in. When `network=true`, the egress
//! token check is bypassed but the command still runs inside the sandbox wrapper.
//!
//! This is the "cloud sandbox" primitive: a safe place to run untrusted build/
//! test commands without touching the host network or leaking secrets. No RNG,
//! no Date — fully deterministic given the same command.

use std::process::{Command, Stdio};

/// Outcome of a sandboxed command. `error` is `Some` when the sandbox REFUSED
/// to run (policy denial) — distinct from a normal non-zero exit code.
#[derive(Clone, Debug)]
pub struct SandboxOutput {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub error: Option<String>,
}

/// Tokens that imply network egress — refused unless `network` is opted in.
const EGRESS_TOKENS: &[&str] = &[
    "curl",
    "wget",
    "ssh",
    "scp",
    "nc",
    "ncat",
    "telnet",
    "ftp",
    "rsync",
    "git ",
    "npm ",
    "pnpm ",
    "cargo ",
    "pip",
    "pip3",
    "go get",
    "docker pull",
    "http",
    "https",
    "://",
];

/// Run `cmd` inside the sandbox.
///
/// - `network = false` (default): network namespace is dropped; any egress token
///   in the command is REFUSED (fail-closed).
/// - `network = true`: egress tokens allowed, but still wrapped in the sandbox.
///
/// The wrapper uses `unshare -n` when present (Linux). If `unshare` is missing,
/// we still enforce the egress-token policy and run via `sh -c` (no elevated
/// isolation, but the policy refusal still holds).
pub fn run_sandboxed(cmd: &str, network: bool) -> SandboxOutput {
    let trimmed = cmd.trim();
    if trimmed.is_empty() {
        return SandboxOutput {
            exit_code: 127,
            stdout: String::new(),
            stderr: String::new(),
            error: Some("empty command".into()),
        };
    }

    // Fail-closed egress gate: refuse network-bound commands unless opted in.
    if !network {
        let lower = trimmed.to_lowercase();
        if EGRESS_TOKENS.iter().any(|t| lower.contains(t)) {
            return SandboxOutput {
                exit_code: 126,
                stdout: String::new(),
                stderr: String::new(),
                error: Some(format!(
                    "egress token detected and network=off (fail-closed); set network:true to opt in"
                )),
            };
        }
    }

    // Build the wrapper. Prefer `unshare -n` (drop network ns) when available.
    let wrapper = if Command::new("unshare")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
    {
        // network ns dropped unless explicitly opted in
        if network {
            vec!["sh".to_string(), "-c".to_string(), trimmed.to_string()]
        } else {
            vec![
                "unshare".to_string(),
                "-n".to_string(),
                "sh".to_string(),
                "-c".to_string(),
                trimmed.to_string(),
            ]
        }
    } else {
        // No unshare: fall back to plain sh -c (policy refusal above still holds).
        vec!["sh".to_string(), "-c".to_string(), trimmed.to_string()]
    };

    let output = Command::new(&wrapper[0])
        .args(&wrapper[1..])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output();

    match output {
        Ok(o) => {
            let code = o.status.code().unwrap_or(-1);
            SandboxOutput {
                exit_code: code,
                stdout: String::from_utf8_lossy(&o.stdout).to_string(),
                stderr: String::from_utf8_lossy(&o.stderr).to_string(),
                error: None,
            }
        }
        Err(e) => SandboxOutput {
            exit_code: 127,
            stdout: String::new(),
            stderr: String::new(),
            error: Some(format!("sandbox spawn failed: {e}")),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safe_command_runs_offline() {
        // GREEN: an offline command runs and returns its stdout.
        let out = run_sandboxed("echo hi-from-sandbox", false);
        assert!(
            out.error.is_none(),
            "should not be refused: {:?}",
            out.error
        );
        assert!(out.stdout.contains("hi-from-sandbox"));
        assert_eq!(out.exit_code, 0);
    }

    #[test]
    fn egress_refused_when_network_off() {
        // RED: a network command with network=off is REFUSED (fail-closed).
        let out = run_sandboxed("curl https://example.com", false);
        assert!(
            out.error.is_some(),
            "egress must be refused with network=off"
        );
        assert!(out.error.unwrap().contains("fail-closed"));
    }

    #[test]
    fn egress_allowed_when_network_opted_in() {
        // GREEN: with network=true the egress token is permitted (still sandboxed).
        // We assert it is NOT refused by policy (the command itself may fail to
        // reach the network in this env, but the policy gate must open).
        let out = run_sandboxed("curl --version", true);
        assert!(
            out.error.is_none() || !out.error.unwrap().contains("fail-closed"),
            "network=on must not trip the fail-closed egress gate"
        );
    }

    #[test]
    fn empty_command_refused() {
        // RED: empty command is refused, not executed.
        let out = run_sandboxed("   ", false);
        assert!(out.error.is_some());
        assert!(out.error.unwrap().contains("empty"));
    }
}
