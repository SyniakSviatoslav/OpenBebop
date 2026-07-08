//! Bebop core — the self-contained deterministic guard kernel, compiled to WASM.
//!
//! This IS the "kernel" the operator wanted inside the bebop repo: the trust boundary that no
//! cloned `bebop.json` can relax. Rust, no clock/RNG/network in the decision path. The CLI calls
//! `decide` via the wasm boundary; if the wasm is absent it falls back to the TS port (parity).
//!
//! NOTE: this is bebop's *own* guard kernel (red-line + scope deny/pass + decision log) — NOT the
//! dowiz food-delivery order state machine that shares the name in the larger monorepo.

use std::sync::OnceLock;

// ── glob → regex (faithful port of guard.ts `toRegExp`) ────────────────────────────────────────
fn glob_to_regex(glob: &str) -> String {
    let mut re = String::new();
    let chars: Vec<char> = glob.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '*' {
            if i + 1 < chars.len() && chars[i + 1] == '*' {
                re.push_str(".*");
                i += 1; // consume second '*'
                if i + 1 < chars.len() && chars[i + 1] == '/' {
                    i += 1; // consume the slash after "**/"
                }
            } else {
                re.push_str("[^/]*");
            }
        } else if c == '?' {
            re.push_str("[^/]");
        } else if ".+^${}()|[]\\".contains(c) {
            re.push('\\');
            re.push(c);
        } else {
            re.push(c);
        }
        i += 1;
    }
    format!("^(?:{re})$")
}

mod glob {
    use std::collections::HashMap;
    use std::sync::OnceLock;

    // caches compiled regexes (Rust's regex::Regex isn't used to keep the wasm tiny + no regex crate
    // dependency — we use a tiny matcher instead, see matches()).
    static CACHE: OnceLock<std::sync::Mutex<HashMap<String, String>>> = OnceLock::new();

    fn cache() -> &'static std::sync::Mutex<HashMap<String, String>> {
        CACHE.get_or_init(|| std::sync::Mutex::new(HashMap::new()))
    }

    /// Glob match faithful to guard.ts's `new RegExp(...).test(p)`:
    /// `**` = any chars (cross-segment), `*` = within one segment, `?` = one char.
    pub fn matches(glob: &str, p: &str) -> bool {
        let mut c = cache().lock().unwrap();
        let regex = c.entry(glob.to_string()).or_insert_with(|| super::glob_to_regex(glob));
        let re = regex.clone();
        drop(c);
        super::regex_test(&re, p)
    }
}

/// Minimal regex tester supporting the subset we generate: `.*`, `[^/]*`, `[^/]`, escaped literals,
/// `^`/`$` anchors, and literal chars. No backtracking needed for these patterns.
fn regex_test(re: &str, p: &str) -> bool {
    // We only generate anchored patterns `^(?:...)$`; strip anchors for the match loop.
    let inner = re.trim_start_matches("^(?:").trim_end_matches(")$");
    backtrack(inner, p, 0, 0)
}

fn backtrack(pat: &str, text: &str, pi: usize, ti: usize) -> bool {
    let pchars: Vec<char> = pat.chars().collect();
    let tchars: Vec<char> = text.chars().collect();
    let plen = pchars.len();
    let tlen = tchars.len();
    let mut p = pi;
    let mut t = ti;
    while p < plen {
        let c = pchars[p];
        if c == '.' && p + 1 < plen && pchars[p + 1] == '*' {
            // .* — greedy match any chars
            p += 2;
            for skip in t..=tlen {
                if backtrack(pat, text, p, skip) {
                    return true;
                }
            }
            return false;
        } else if c == '[' && p + 1 < plen && pchars[p + 1] == '^' {
            // [^/] or [^...] — one char not in the set; with a trailing '*' it becomes 0+.
            let close = (p + 2..plen).find(|&i| pchars[i] == ']').unwrap_or(plen);
            let star = close + 1 < plen && pchars[close + 1] == '*';
            let set: String = pchars[p + 2..close].iter().collect();
            if t >= tlen && !star {
                return false;
            }
            if set == "/" {
                if star {
                    // 0+ non-slash chars (greedy, then backtrack)
                    let mut consumed = 0;
                    while t + consumed < tlen && tchars[t + consumed] != '/' {
                        consumed += 1;
                    }
                    for k in (0..=consumed).rev() {
                        if backtrack(pat, text, close + 2, t + k) {
                            return true;
                        }
                    }
                    return false;
                } else if tchars[t] == '/' {
                    return false;
                }
            } else if !star {
                // only the single-char negation sets we emit here
                if set.chars().any(|s| s == tchars[t]) {
                    return false;
                }
            } else {
                // 0+ chars not in set (greedy, then backtrack)
                let mut consumed = 0;
                while t + consumed < tlen && !set.chars().any(|s| s == tchars[t + consumed]) {
                    consumed += 1;
                }
                for k in (0..=consumed).rev() {
                    if backtrack(pat, text, close + 2, t + k) {
                        return true;
                    }
                }
                return false;
            }
            p = close + 1 + if star { 1 } else { 0 };
            t += 1;
        } else if c == '\\' && p + 1 < plen {
            // escaped literal
            p += 1;
            if t >= tlen || pchars[p] != tchars[t] {
                return false;
            }
            p += 1;
            t += 1;
        } else {
            if t >= tlen || pchars[p] != tchars[t] {
                return false;
            }
            p += 1;
            t += 1;
        }
    }
    t == tlen
}

// ── red-line + scope globs (mirror of guard.ts) ────────────────────────────────────────────────
pub fn red_line_globs() -> Vec<String> {
    vec![
        "**/auth/**",
        "**/migrations/**",
        "**/rls/**",
        "**/*.sql",
        "**/packages/db/migrations/**",
        "**/money/**",
        "**/payments/**",
        "**/bulk-edit/**",
        "**/secret/**",
        "**/secrets/**",
        "**/.env",
        "**/.env.*",
    ]
    .into_iter()
    .map(String::from)
    .collect()
}

pub fn default_scope_globs() -> Vec<String> {
    vec!["tools/bebop/**", "docs/design/dowiz-agent-cli/**"]
        .into_iter()
        .map(String::from)
        .collect()
}

// ── decide ─────────────────────────────────────────────────────────────────────────────────────
#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct Decision {
    pub ok: bool,
    pub kind: String, // "redline" | "scope" | "ok"
    pub reason: String,
    pub deny: bool,
}

/// Decide on a target path + operation. `op` = "read" | "edit" | "run" | "dispatch" | "recall".
/// `extra_deny` strengthens red-lines (user settings only). `scope` narrows allowed surface.
pub fn decide(
    target: &str,
    op: &str,
    extra_deny: &[String],
    scope: &[String],
    cwd: &str,
) -> Decision {
    // 1. red-line deny (hardcoded core — cannot be relaxed)
    for g in red_line_globs() {
        if glob::matches(&g, target) {
            return Decision {
                ok: false,
                kind: "redline".into(),
                reason: "red-line: requires explicit human go-ahead (auth/money/RLS/migrations/secrets).".into(),
                deny: true,
            };
        }
    }
    // 2. user-supplied deny globs (strengthen only)
    for g in extra_deny {
        if glob::matches(g, target) {
            return Decision {
                ok: false,
                kind: "redline".into(),
                reason: "red-line (user deny): blocked by ~/.bebop/settings.json.".into(),
                deny: true,
            };
        }
    }
    // 3. scope check (only for file-bearing ops)
    if matches!(op, "read" | "edit" | "run" | "dispatch") && !scope.is_empty() {
        let rel = if target.starts_with('/') {
            strip_prefix(cwd, target)
        } else {
            target.to_string()
        };
        let candidates = [target, rel.as_str()];
        let allowed = scope.iter().any(|g| candidates.iter().any(|c| glob::matches(g, c)));
        if !allowed {
            return Decision {
                ok: false,
                kind: "scope".into(),
                reason: "scope: outside the agreed surface; re-ask before touching.".into(),
                deny: true,
            };
        }
    }
    Decision {
        ok: true,
        kind: "ok".into(),
        reason: String::new(),
        deny: false,
    }
}

fn strip_prefix(cwd: &str, abs: &str) -> String {
    if let Some(suffix) = abs.strip_prefix(cwd) {
        suffix.trim_start_matches('/').to_string()
    } else {
        abs.to_string()
    }
}

// ── append-only decision log (the immutable kernel memory of what it refused) ──────────────────
use std::sync::Mutex;

pub struct Kernel {
    log: Mutex<Vec<String>>,
}

static KERNEL: OnceLock<Kernel> = OnceLock::new();

fn kernel() -> &'static Kernel {
    KERNEL.get_or_init(|| Kernel { log: Mutex::new(Vec::new()) })
}

/// Record a decision line (JSON) into the append-only log. Returns the seq number.
pub fn record(target: &str, op: &str, decision: &Decision) -> u64 {
    let k = kernel();
    let mut log = k.log.lock().unwrap();
    let seq = log.len() as u64;
    let line = serde_json::json!({
        "seq": seq,
        "op": op,
        "target": target,
        "ok": decision.ok,
        "kind": decision.kind,
    })
    .to_string();
    log.push(line);
    seq
}

pub fn log_len() -> u64 {
    kernel().log.lock().unwrap().len() as u64
}

// ── retriever (VSA port: deterministic hash embeddings, no network) ────────────────────────────
pub mod retriever {
    use super::*;

    /// FNV-1a 64-bit hash — deterministic, no deps.
    pub fn fnv1a(s: &str) -> u64 {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        for b in s.bytes() {
            h ^= b as u64;
            h = h.wrapping_mul(0x0000_0100_0000_01b3);
        }
        h
    }

    /// Build a fixed-dim binary VSA vector from text (token hashes → bipolar). Deterministic.
    pub fn embed(text: &str, dim: usize) -> Vec<i8> {
        let mut v = vec![0i8; dim];
        let tokens: Vec<&str> = text.split_whitespace().collect();
        if tokens.is_empty() {
            return v;
        }
        for (i, tok) in tokens.iter().cycle().take(dim).enumerate() {
            let h = fnv1a(tok);
            v[i] = if (h & 1) == 1 { 1 } else { -1 };
        }
        v
    }

    /// Cosine-over-bipolar similarity in [-1, 1].
    pub fn similarity(a: &[i8], b: &[i8]) -> f64 {
        if a.is_empty() || b.is_empty() || a.len() != b.len() {
            return 0.0;
        }
        let dot: i32 = a.iter().zip(b).map(|(x, y)| (*x as i32) * (*y as i32)).sum();
        dot as f64 / a.len() as f64
    }

    pub fn estimate_tokens(text: &str) -> usize {
        // ~4 chars/token heuristic; deterministic.
        (text.chars().count() + 3) / 4
    }

    // ── wasm C-ABI surface (hand-rolled, no wasm-bindgen → tiny, zero host deps) ────────────────
    // Pattern: calls that return strings write into a global buffer; the host reads it via
    // bebop_result_ptr()/bebop_result_len() then copies it out. No malloc coordination needed.
    // Mutex-wrapped so concurrent wasm calls (single-threaded, but sound) can't create UB.
    static RESULT: std::sync::Mutex<String> = std::sync::Mutex::new(String::new());

    fn set_result(s: String) {
        *RESULT.lock().unwrap() = s;
    }

    #[no_mangle]
    pub extern "C" fn bebop_result_ptr() -> *const u8 {
        RESULT.lock().unwrap().as_ptr()
    }

    #[no_mangle]
    pub extern "C" fn bebop_result_len() -> usize {
        RESULT.lock().unwrap().len()
    }

    /// Decide from JSON args. `args` = {"target","op","extra_deny":[..],"scope":[..],"cwd"}.
    /// Result written to the shared buffer as JSON Decision.
    #[no_mangle]
    pub extern "C" fn bebop_decide(args: *const u8, len: usize) {
        let bytes = unsafe { std::slice::from_raw_parts(args, len) };
        let v: serde_json::Value = match serde_json::from_slice(bytes) {
            Ok(v) => v,
            Err(e) => {
                set_result(format!("{{\"ok\":false,\"kind\":\"error\",\"reason\":\"{e}\",\"deny\":true}}"));
                return;
            }
        };
        let target = v["target"].as_str().unwrap_or("");
        let op = v["op"].as_str().unwrap_or("edit");
        let extra: Vec<String> = v["extra_deny"]
            .as_array()
            .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
            .unwrap_or_default();
        let scope: Vec<String> = v["scope"]
            .as_array()
            .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
            .unwrap_or_default();
        let cwd = v["cwd"].as_str().unwrap_or("");
        let d = decide(target, op, &extra, &scope, cwd);
        record(target, op, &d);
        set_result(serde_json::to_string(&d).unwrap_or_else(|_| "{\"ok\":false}".into()));
    }

    /// Embed text → JSON array of i8. `args` = {"text","dim"}.
    #[no_mangle]
    pub extern "C" fn bebop_embed(args: *const u8, len: usize) {
        let bytes = unsafe { std::slice::from_raw_parts(args, len) };
        let v: serde_json::Value = serde_json::from_slice(bytes).unwrap_or(serde_json::json!({}));
        let text = v["text"].as_str().unwrap_or("");
        let dim = v["dim"].as_u64().unwrap_or(256) as usize;
        let emb = embed(text, dim);
        set_result(serde_json::to_string(&emb).unwrap_or_else(|_| "[]".into()));
    }

    /// Similarity of two JSON i8 arrays. `args` = {"a":[..],"b":[..]}.
    #[no_mangle]
    pub extern "C" fn bebop_similarity(args: *const u8, len: usize) {
        let bytes = unsafe { std::slice::from_raw_parts(args, len) };
        let v: serde_json::Value = serde_json::from_slice(bytes).unwrap_or(serde_json::json!({}));
        let to_vec = |key: &str| -> Vec<i8> {
            v[key]
                .as_array()
                .map(|a| a.iter().filter_map(|x| x.as_i64().map(|n| n as i8)).collect())
                .unwrap_or_default()
        };
        let s = similarity(&to_vec("a"), &to_vec("b"));
        set_result(format!("{s}"));
    }

    /// Estimate tokens for text. Returns the count as a JSON number string.
    #[no_mangle]
    pub extern "C" fn bebop_estimate_tokens(args: *const u8, len: usize) {
        let bytes = unsafe { std::slice::from_raw_parts(args, len) };
        let v: serde_json::Value = serde_json::from_slice(bytes).unwrap_or(serde_json::json!({}));
        let n = estimate_tokens(v["text"].as_str().unwrap_or(""));
        set_result(format!("{n}"));
    }

    /// Append-only log length.
    #[no_mangle]
    pub extern "C" fn bebop_log_len() -> u64 {
        log_len()
    }

    /// Export the decision log as a JSON array string.
    #[no_mangle]
    pub extern "C" fn bebop_export_log() {
        let k = kernel();
        let log = k.log.lock().unwrap();
        set_result(serde_json::to_string(&*log).unwrap_or_else(|_| "[]".into()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn red_line_denies_migrations_and_secrets() {
        assert!(!decide("packages/db/migrations/002_users.sql", "edit", &[], &[], "").ok);
        assert!(!decide(".env", "read", &[], &[], "").ok);
        assert!(!decide("secret/key.txt", "read", &[], &[], "").ok);
    }

    #[test]
    fn green_allows_tool_files() {
        let d = decide("tools/bebop/src/loop.ts", "edit", &[], &["tools/bebop/**".to_string()], "/repo");
        assert!(d.ok, "tool file must be allowed");
    }

    #[test]
    fn user_deny_strengthens_not_relaxes() {
        // user deny adds a glob; a previously-allowed path is now refused
        assert!(decide("src/experimental.ts", "edit", &[], &[], "").ok);
        assert!(!decide("src/experimental.ts", "edit", &["**/experimental.ts".into()], &[], "").ok);
    }

    #[test]
    fn scope_blocks_outside_surface() {
        let d = decide("apps/api/server.ts", "edit", &[], &["tools/bebop/**".to_string()], "/repo");
        assert!(!d.ok);
        assert_eq!(d.kind, "scope");
    }

    #[test]
    fn retriever_embed_is_deterministic_and_similar() {
        let a = retriever::embed("the red ship lifts off", 256);
        let b = retriever::embed("the red ship lifts off", 256);
        let c = retriever::embed("unrelated coffee morning", 256);
        assert_eq!(a, b, "same input → same vector");
        assert!(retriever::similarity(&a, &b) > retriever::similarity(&a, &c));
    }

    #[test]
    fn glob_stars_and_wildcards() {
        assert!(glob::matches("**/auth/**", "x/y/auth/token.ts"));
        assert!(glob::matches("**/*.sql", "a/b/migration.sql"));
        assert!(!glob::matches("**/secret/**", "src/secret.ts")); // not a directory
    }

    #[test]
    fn log_records_decisions() {
        let before = log_len();
        let d = decide("migrations/x.sql", "edit", &[], &[], "");
        record("migrations/x.sql", "edit", &d);
        assert_eq!(log_len(), before + 1);
    }
}
