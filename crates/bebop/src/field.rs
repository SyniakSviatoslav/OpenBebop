//! Field — the deterministic graph-PDE arbiter (the "physics veto").
//!
//! The real field core lives in `rust-core/` (dependency-free, air-gapped): a
//! spectral heat-kernel propagator over a dependency graph. The cost surface it
//! produces is the arbiter: a plan that would dump significant mass onto the
//! red-line (secrets) node is VETOED. This module is the host-side handle that
//! builds a deterministic plan graph and returns the verdict.
//!
//! No RNG, no Date, no network — fully reproducible (same plan → same verdict).

use bebop_core;
use std::sync::Mutex;

/// The `rust-core` field C-API keeps its graph in PROCESS-GLOBAL state
/// (`field_build`/`field_reset` mutate statics). Concurrent calls from parallel
/// `#[test]` threads would race, so every field-core sequence is serialized
/// behind this lock. Deterministic + thread-safe.
static FIELD_LOCK: Mutex<()> = Mutex::new(());

/// Run the full field-core sequence (build → rank → reset) under the lock and
/// return `out` for node `node`. `None` if the build failed.
fn field_eval(node: usize, n: i32, row: &[i32], col: &[i32]) -> Option<Vec<f64>> {
    let _guard = FIELD_LOCK.lock().unwrap();
    let rc = unsafe { bebop_core::field_build(row.as_ptr(), col.as_ptr(), col.len() as i32, n) };
    if rc != 0 {
        return None;
    }
    let nn = n as usize;
    let mut seed = vec![0.0f64; nn];
    seed[node] = 1.0;
    let mut out = vec![0.0f64; nn];
    unsafe {
        bebop_core::field_rank(
            seed.as_ptr(),
            std::ptr::null(),
            1.0,
            0.5,
            20,
            out.as_mut_ptr(),
        );
    }
    unsafe { bebop_core::field_reset() };
    Some(out)
}

/// Build a small deterministic plan graph as CSR (undirected Laplacian L = D − A).
/// Nodes: 0=plan, 1=impl, 2=test, 3=deploy, 4=secrets(red-line), 5=docs.
/// Edges: plan↔impl, impl↔test, test↔deploy, deploy↔docs, deploy↔secrets.
fn plan_csr() -> (Vec<i32>, Vec<i32>, i32) {
    let edges: &[(usize, usize)] = &[(0, 1), (1, 2), (2, 3), (3, 4), (3, 5)];
    let n = 6;
    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); n];
    for &(a, b) in edges {
        adj[a].push(b);
        adj[b].push(a);
    }
    let mut row = vec![0i32; n + 1];
    for i in 0..n {
        row[i + 1] = row[i] + adj[i].len() as i32;
    }
    let mut col = Vec::with_capacity(row[n] as usize);
    for i in 0..n {
        for &j in &adj[i] {
            col.push(j as i32);
        }
    }
    (row, col, n as i32)
}

/// The arbiter verdict for an action that would disrupt `node`.
/// Returns "override" (vetoed: blast on red-line node > tolerance) or "permit".
pub fn field_gate(task: &str) -> String {
    // Map task text to the node it would disrupt (deterministic keyword map).
    let node = if task.contains("secret")
        || task.contains("auth")
        || task.contains("money")
        || task.contains("migrat")
        || task.contains("rls")
    {
        4 // secrets / red-line node — touching it is the highest cost
    } else if task.contains("deploy") {
        3
    } else if task.contains("test") {
        2
    } else if task.contains("doc") {
        5
    } else {
        1 // default: implementation
    };

    const SECRETS: usize = 4;
    let (row, col, n) = plan_csr();
    let out = match field_eval(node, n, &row, &col) {
        Some(o) => o,
        None => return "permit".into(), // build failed → fail-open at field layer
    };

    let blast_on_secrets = out[SECRETS];
    // Tolerance: a disruption whose predicted mass on the red-line node exceeds it
    // is vetoed by the field. Deterministic + falsifiable.
    const TOLERANCE: f64 = 0.10;
    if blast_on_secrets > TOLERANCE {
        "override".into()
    } else {
        "permit".into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redline_task_is_vetoed() {
        // RED+GREEN: a task that would touch the secrets/red-line node must be OVERRIDDEN
        // by the field arbiter (physics veto), proving the veto path is live.
        assert_eq!(field_gate("rotate the deploy secrets"), "override");
        assert_eq!(field_gate("edit auth login flow"), "override");
    }

    #[test]
    fn benign_task_is_permitted() {
        // GREEN: a normal implementation/doc task stays permitted (not over-vetoed).
        assert_eq!(field_gate("write the docs"), "permit");
        assert_eq!(field_gate("implement the parser"), "permit");
    }

    #[test]
    fn verdict_is_deterministic() {
        // GREEN/RED: same task yields the same verdict every call.
        assert_eq!(
            field_gate("rotate the deploy secrets"),
            field_gate("rotate the deploy secrets")
        );
        assert_eq!(field_gate("write the docs"), field_gate("write the docs"));
    }

    #[test]
    fn blast_threshold_is_real() {
        // RED: a disruption ON the secrets node dumps ~0.66 mass on it (≫ tolerance)
        // while a docs disruption dumps only ~0.06 (≪ tolerance). Prove the gap.
        let (row, col, n) = plan_csr();
        let secrets = field_eval(4, n, &row, &col).expect("field build");
        assert!(secrets[4] > 0.5, "secrets blast should be >> tolerance");

        let docs = field_eval(5, n, &row, &col).expect("field build");
        assert!(
            docs[4] < 0.10,
            "docs blast on secrets should be under tolerance"
        );
    }
}
