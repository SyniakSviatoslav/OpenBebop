// rust-core/examples/metrics_probe.rs
// OPTION-#1 OBSERVABILITY sample: build a small path graph, run a propagation, then read the
// kernel's numeric state via `field_metrics` (C-ABI). Prints the 5-tuple the host telemetry
// loop consumes. Doubles as a self-proving smoke test of the metric surface.
//
//   cargo run --example metrics_probe
//
// Output (single line, space-separated):
//   metrics: count=<n> sum_energy=<f> max_energy=<f> mean_energy=<f> nodes=<n>

fn main() {
    // 8-node path graph: edges (i,i+1)
    let n = 8i32;
    let mut rp = vec![0i32; (n + 1) as usize];
    let mut ci = Vec::new();
    let mut e = 0i32;
    for i in 0..n {
        if i > 0 {
            ci.push(i - 1);
            e += 1;
        }
        if i < n - 1 {
            ci.push(i + 1);
            e += 1;
        }
        rp[(i + 1) as usize] = e;
    }
    unsafe {
        let rc = bebop_core::field_build(rp.as_ptr(), ci.as_ptr(), e, n);
        assert_eq!(rc, 0, "field_build failed");
    }

    let mut u0 = [0.0f64; 8];
    u0[0] = 1.0;
    let mut out = [0.0f64; 8];
    unsafe {
        // two propagations so count > 0 and energy accrues
        bebop_core::field_spectral(u0.as_ptr(), 5.0, 1.0, 30, out.as_mut_ptr());
        bebop_core::field_spectral(u0.as_ptr(), 5.0, 1.0, 30, out.as_mut_ptr());
    }

    // also exercise the bridge cargos so their call counters increment (lightweight tallies)
    let mut rank_out = [0.0f64; 8];
    unsafe {
        bebop_core::field_rank(
            u0.as_ptr(),
            core::ptr::null(),
            5.0,
            1.0,
            30,
            rank_out.as_mut_ptr(),
        );
        let _ = bebop_core::field_cost(u0.as_ptr(), core::ptr::null(), 5.0, 1.0, 30);
    }

    let mut m = [0.0f64; 12];
    let rc = unsafe { bebop_core::field_metrics(m.as_mut_ptr(), 12) };
    assert_eq!(rc, 0, "field_metrics failed");
    println!(
        "metrics: count={} sum_dU={:.6} max_dU={:.6} mean_dU={:.6} nodes={} E_last={:.6} E0={:.6} stabilize_ratio={:.6} rank={} cost={} spectral={} active={}",
        m[0] as i64, m[1], m[2], m[3], m[4] as i64, m[5], m[6], m[7],
        m[8] as i64, m[9] as i64, m[10] as i64, m[11] as i64
    );
    assert!(m[0] >= 2.0, "expected >=2 propagations recorded");
    assert!(m[1] > 0.0, "expected positive total energy");
    // ENERGY DOCTRINE: contractive field ⇒ stabilize_ratio ≤ 1 (relaxes toward ZERO ground state)
    assert!(
        m[7] <= 1.0 + 1e-9,
        "stabilize_ratio must be <= 1, got {}",
        m[7]
    );
    // LIGHTWEIGHT COUNTERS: rank/cost were each called once above
    assert_eq!(m[8] as i64, 1, "rank_calls must be 1");
    assert_eq!(m[9] as i64, 1, "cost_calls must be 1");
    assert_eq!(m[10] as i64, 2, "spectral_calls must be 2");
}
