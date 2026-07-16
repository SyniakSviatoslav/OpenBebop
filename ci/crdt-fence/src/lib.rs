//! MESH-08 CRDT-periphery compile-fence.
//!
//! Makes "CRDT never for money/orders" a COMPILE-TIME invariant rather than a
//! doc convention: this lint walks the `cargo metadata` dependency graph of the
//! order/money-adjacent crates and FAILS if any path reaches a crate whose name
//! matches a CRDT-merge pattern (automerge / cr-sqlite / merge-crdt / *crdt*).
//!
//! The tool depends only on `serde_json` (a dev-only parser) and on NO CRDT crate.

use serde_json::Value;
use std::collections::{HashMap, HashSet};

/// Crates whose dependency graph must NEVER reach a CRDT-merge crate.
/// These touch order/money state (or the protocol lines that carry it).
pub const GUARDED_CRATES: &[&str] = &[
    "bebop2-core",
    "bebop",
    "bebop-delivery-domain",
    "bebop-mesh-node",
];

/// Case-insensitive regex matching CRDT-merge crates we forbid money/order crates
/// from depending on (transitively).
pub const CRDT_PATTERN: &str = r"(?i)crdt|automerge|cr-sqlite|merge-crdt";

/// One offending path through the dependency graph from a guarded crate to a CRDT crate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Offense {
    pub guarded: String,
    pub crdt: String,
    pub path: Vec<String>,
}

/// Parse a `cargo metadata --format-version=1` JSON document and return every
/// offending (guarded_crate -> CRDT crate) path, walking the full transitive
/// dependency graph. Returns an empty vec when the graph is clean.
pub fn find_offenses(metadata: &str) -> Result<Vec<Offense>, String> {
    let root: Value = serde_json::from_str(metadata).map_err(|e| format!("invalid JSON: {e}"))?;

    // Package id -> display name. Resolved from the `packages` array (NOT the id
    // string, because path-dependency ids omit the package name).
    let pkgs = root
        .get("packages")
        .and_then(Value::as_array)
        .ok_or_else(|| "missing `packages` array".to_string())?;
    let mut name_by_id: HashMap<String, String> = HashMap::new();
    for p in pkgs {
        let id = p.get("id").and_then(Value::as_str).unwrap_or("").to_string();
        let name = p.get("name").and_then(Value::as_str).unwrap_or("").to_string();
        if !id.is_empty() {
            name_by_id.insert(id, name);
        }
    }

    // Build adjacency from the resolve graph: node.id -> [dependency ids].
    let resolve = root
        .get("resolve")
        .ok_or_else(|| "missing `resolve`".to_string())?;
    let nodes = resolve
        .get("nodes")
        .and_then(Value::as_array)
        .ok_or_else(|| "missing `resolve.nodes`".to_string())?;

    let mut deps_of: HashMap<String, Vec<String>> = HashMap::new();
    for n in nodes {
        let id = n.get("id").and_then(Value::as_str).unwrap_or("").to_string();
        let mut out = Vec::new();
        if let Some(arr) = n.get("dependencies").and_then(Value::as_array) {
            for d in arr {
                if let Some(s) = d.as_str() {
                    out.push(s.to_string());
                }
            }
        }
        deps_of.insert(id, out);
    }

    let name_of = |id: &str| -> String {
        name_by_id
            .get(id)
            .cloned()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| id.to_string())
    };

    let crdt_re = regex::build(CRDT_PATTERN);

    let mut offenses = Vec::new();
    let guarded_ids: Vec<String> = name_by_id
        .iter()
        .filter(|(_, n)| GUARDED_CRATES.iter().any(|g| *g == n.as_str()))
        .map(|(id, _)| id.clone())
        .collect();

    for start in &guarded_ids {
        // DFS over the dependency graph, tracking the path for a clear report.
        let mut visited: HashSet<String> = HashSet::new();
        let mut stack: Vec<(String, Vec<String>)> = vec![(start.clone(), vec![name_of(start)])];
        while let Some((cur, path)) = stack.pop() {
            if !visited.insert(cur.clone()) {
                continue; // already explored this node along another branch
            }
            let cn = name_of(&cur);
            if crdt_re.is_match(&cn) {
                offenses.push(Offense {
                    guarded: name_of(start),
                    crdt: cn,
                    path,
                });
                // We found a CRDT dependency for this guarded crate; reporting one
                // representative shortest-ish path is sufficient — stop this DFS.
                break;
            }
            for dep in deps_of.get(&cur).cloned().unwrap_or_default() {
                if !visited.contains(&dep) {
                    let mut p = path.clone();
                    p.push(name_of(&dep));
                    stack.push((dep, p));
                }
            }
        }
    }

    // Stable ordering for deterministic test output.
    offenses.sort_by(|a, b| a.guarded.cmp(&b.guarded).then(a.crdt.cmp(&b.crdt)));
    Ok(offenses)
}

/// Inline, tiny, dependency-free, case-insensitive substring/alternation match.
/// Supports the fixed pattern we use: `crdt | automerge | cr-sqlite | merge-crdt`,
/// all case-insensitively. We avoid pulling a regex crate into this dev tool.
mod regex {
    pub struct Matcher;

    impl Matcher {
        pub fn is_match(&self, hay: &str) -> bool {
            // The only alternation we need; keep it explicit and auditable.
            contains_ci(hay, "crdt")
                || contains_ci(hay, "automerge")
                || contains_ci(hay, "cr-sqlite")
                || contains_ci(hay, "merge-crdt")
        }
    }

    pub fn build(_pattern: &str) -> Matcher {
        Matcher
    }

    fn contains_ci(hay: &str, needle: &str) -> bool {
        let h: Vec<char> = hay.chars().collect();
        let n: Vec<char> = needle.chars().collect();
        if n.is_empty() || h.len() < n.len() {
            return false;
        }
        let mut lower = Vec::with_capacity(h.len());
        for c in &h {
            lower.push(c.to_ascii_lowercase());
        }
        let nl: Vec<char> = n.iter().map(|c| c.to_ascii_lowercase()).collect();
        for w in 0..=(h.len() - n.len()) {
            if lower[w..w + n.len()] == nl[..] {
                return true;
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fake `cargo metadata` JSON with NO CRDT deps reachable from a guarded crate.
    const CLEAN_METADATA: &str = r#"{
      "packages": [
        {"id": "p#bebop2-core@0.1.0", "name": "bebop2-core"},
        {"id": "p#bebop@0.4.0",       "name": "bebop"},
        {"id": "p#serde_json@1.0",    "name": "serde_json"},
        {"id": "p#tokio@1.0",         "name": "tokio"}
      ],
      "resolve": {
        "nodes": [
          {"id": "p#bebop2-core@0.1.0", "dependencies": []},
          {"id": "p#bebop@0.4.0",       "dependencies": ["p#bebop2-core@0.1.0", "p#serde_json@1.0"]},
          {"id": "p#serde_json@1.0",    "dependencies": []},
          {"id": "p#tokio@1.0",         "dependencies": []}
        ]
      }
    }"#;

    /// A fake `cargo metadata` JSON where a guarded crate DOES reach a CRDT crate.
    const DIRTY_METADATA: &str = r#"{
      "packages": [
        {"id": "p#bebop-delivery-domain@0.1.0", "name": "bebop-delivery-domain"},
        {"id": "p#automerge@0.5",                "name": "automerge"},
        {"id": "p#automerge-repo@0.1",           "name": "automerge-repo"}
      ],
      "resolve": {
        "nodes": [
          {"id": "p#bebop-delivery-domain@0.1.0", "dependencies": ["p#automerge@0.5"]},
          {"id": "p#automerge@0.5",                "dependencies": ["p#automerge-repo@0.1"]},
          {"id": "p#automerge-repo@0.1",           "dependencies": []}
        ]
      }
    }"#;

    /// cr-sqlite (hyphenated variant) must also be caught.
    const DIRTY_SQLITE_METADATA: &str = r#"{
      "packages": [
        {"id": "p#bebop-mesh-node@0.1.0", "name": "bebop-mesh-node"},
        {"id": "r#cr-sqlite@0.1",         "name": "cr-sqlite"}
      ],
      "resolve": {
        "nodes": [
          {"id": "p#bebop-mesh-node@0.1.0", "dependencies": ["r#cr-sqlite@0.1"]},
          {"id": "r#cr-sqlite@0.1",         "dependencies": []}
        ]
      }
    }"#;

    #[test]
    fn red_clean_graph_has_no_offenses() {
        let offenses = find_offenses(CLEAN_METADATA).expect("parse clean");
        assert!(offenses.is_empty(), "expected clean graph, got: {offenses:?}");
    }

    #[test]
    fn red_injected_automerge_dep_is_offense() {
        let offenses = find_offenses(DIRTY_METADATA).expect("parse dirty");
        assert_eq!(offenses.len(), 1, "expected exactly one offense");
        assert_eq!(offenses[0].guarded, "bebop-delivery-domain");
        assert_eq!(offenses[0].crdt, "automerge");
        assert_eq!(
            offenses[0].path,
            vec!["bebop-delivery-domain", "automerge"]
        );
    }

    #[test]
    fn red_injected_cr_sqlite_dep_is_offense() {
        let offenses = find_offenses(DIRTY_SQLITE_METADATA).expect("parse dirty sqlite");
        assert_eq!(offenses.len(), 1);
        assert_eq!(offenses[0].guarded, "bebop-mesh-node");
        assert_eq!(offenses[0].crdt, "cr-sqlite");
    }

    #[test]
    fn invalid_json_errors() {
        assert!(find_offenses("not json").is_err());
    }
}
