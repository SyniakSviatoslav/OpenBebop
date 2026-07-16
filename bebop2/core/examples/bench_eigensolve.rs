//! bench_eigensolve — the "atomic benchmark" PoC (rewrite-roadmap S0.5).
//!
//! Speedometer for the consolidated spectral organ: times the eigenvalue path
//! (authoritative `linalg::eigenvalues`, as reached via `field::jacobi_eigen`'s
//! eigenvalue half) against the general eigensolver (`lyapunov::eigenvalues_general`)
//! on growing graph Laplacians, prints ns (speed), and asserts they AGREE within
//! 1e-9 (the cross-authority parity that markov.rs-style drift would break).
//!
//! Run:  cargo run -p bebop-core --example bench_eigensolve --release
//! (debug build also works; --release gives the real speedometer numbers.)

use bebop2_core::field::jacobi_eigen;
use bebop2_core::lyapunov::eigenvalues_general;
use bebop2_core::speedometer::{bench_ns, shannon_entropy_norm};

/// Path-graph Laplacian L = D - A of `n` nodes, row-major flat.
fn path_laplacian(n: usize) -> Vec<f64> {
    let mut l = vec![0.0f64; n * n];
    for i in 0..n {
        let mut deg = 0.0;
        if i > 0 {
            l[i * n + (i - 1)] = -1.0;
            deg += 1.0;
        }
        if i + 1 < n {
            l[i * n + (i + 1)] = -1.0;
            deg += 1.0;
        }
        l[i * n + i] = deg;
    }
    l
}

fn sorted(mut v: Vec<f64>) -> Vec<f64> {
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    v
}

fn main() {
    println!("bebop eigensolve speedometer (atomic benchmark S0.5)");
    println!(
        "{:>5} {:>14} {:>14} {:>10} {:>12}",
        "n", "jacobi_ns", "general_ns", "max|Δλ|", "H(λ) bits"
    );
    println!("{}", "-".repeat(60));

    for &n in &[4usize, 8, 16, 32] {
        let l = path_laplacian(n);

        // speed: mean ns over repeated solves
        let (jac_ns, _, _) = bench_ns(
            || {
                let _ = jacobi_eigen(&l, n);
            },
            200,
        );
        let (gen_ns, _, _) = bench_ns(
            || {
                let _ = eigenvalues_general(&l, n);
            },
            200,
        );

        // parity: the two solvers MUST agree on the symmetric Laplacian
        let (jac_vals, _) = jacobi_eigen(&l, n);
        let gen_vals: Vec<f64> = eigenvalues_general(&l, n).iter().map(|c| c.re).collect();
        let js = sorted(jac_vals.clone());
        let gs = sorted(gen_vals);
        let max_d = js
            .iter()
            .zip(gs.iter())
            .map(|(a, b)| (a - b).abs())
            .fold(0.0f64, f64::max);
        assert!(
            max_d < 1e-9,
            "DIVERGENCE at n={n}: max|Δλ|={max_d:e} — eigensolvers disagree"
        );

        // entropy gauge: spread of the (shifted-nonneg) spectrum
        let shifted: Vec<f64> = js.iter().map(|&x| x + 1e-12).collect();
        let h = shannon_entropy_norm(&shifted);

        println!("{n:>5} {jac_ns:>14.1} {gen_ns:>14.1} {max_d:>10.2e} {h:>12.4}");
    }

    println!("\nOK: symmetric eigensolvers agree within 1e-9 across all sizes.");
}
