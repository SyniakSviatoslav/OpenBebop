//! speedometer — zero-dep instrumentation for the "benchmark-as-speedometer"
//! invariant (MASTER rewrite-roadmap §1.5.7): speed is the default state, gated
//! in CI; a 12ns→13ns regression must be explained. Also surfaces an entropy
//! gauge where randomness is measurable (operator's explicit tracking ask).
//!
//! No external deps (no criterion) so it builds offline and runs anywhere.
//! Criterion can be layered later as a dev-dependency; this is the floor.

/// Wall-clock a closure `iters` times; return (mean_ns, min_ns, max_ns).
/// Uses std::time::Instant only — deterministic enough for relative regression
/// detection (not a statistical claim; run under `cargo run --example` for PoC).
pub fn bench_ns<F: FnMut()>(mut f: F, iters: u32) -> (f64, f64, f64) {
    let mut min = f64::INFINITY;
    let mut max = 0.0f64;
    let mut sum = 0.0f64;
    for _ in 0..iters {
        let t0 = std::time::Instant::now();
        f();
        let dt = t0.elapsed().as_nanos() as f64;
        min = min.min(dt);
        max = max.max(dt);
        sum += dt;
    }
    if iters == 0 {
        (0.0, 0.0, 0.0)
    } else {
        (sum / iters as f64, min, max)
    }
}

/// Shannon entropy of a byte stream, in bits/byte (max 8.0).
/// A measure of "how random" a sample is — the operator's entropy tracking ask.
/// Returns 0.0 for empty input (undefined otherwise).
pub fn shannon_entropy_bytes(bytes: &[u8]) -> f64 {
    if bytes.is_empty() {
        return 0.0;
    }
    let mut counts = [0u64; 256];
    for &b in bytes {
        counts[b as usize] += 1;
    }
    let n = bytes.len() as f64;
    let mut h = 0.0f64;
    for &c in counts.iter() {
        if c > 0 {
            let p = c as f64 / n;
            h -= p * p.log2();
        }
    }
    h // bits per byte; ≤ 8.0
}

/// Normalized entropy of a slice of f64 in [0,1] (treated as a probability-ish
/// distribution after L1 normalization). Useful for telemetry/loss landscapes
/// where you want "how spread" a vector is without a byte source.
pub fn shannon_entropy_norm(xs: &[f64]) -> f64 {
    let total: f64 = xs.iter().sum();
    if total <= 0.0 {
        return 0.0;
    }
    let mut h = 0.0f64;
    for &x in xs {
        if x > 0.0 {
            let p = x / total;
            h -= p * p.log2();
        }
    }
    h // bits; ≤ log2(len)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bench_ns_runs() {
        let (mean, min, max) = bench_ns(
            || {
                let mut s = 0u64;
                for i in 0..1000 {
                    s = s.wrapping_add(i);
                }
                std::hint::black_box(s);
            },
            50,
        );
        assert!(mean >= 0.0 && min <= mean && max >= mean);
    }

    #[test]
    fn entropy_uniform_max() {
        // All 256 byte values equally present ⇒ 8.0 bits/byte.
        let buf: Vec<u8> = (0u16..=255).map(|b| b as u8).collect();
        let h = shannon_entropy_bytes(&buf);
        assert!((h - 8.0).abs() < 1e-9, "uniform byte stream entropy = 8.0");
    }

    #[test]
    fn entropy_constant_zero() {
        let buf = [42u8; 64];
        assert!((shannon_entropy_bytes(&buf) - 0.0).abs() < 1e-12);
    }

    #[test]
    fn entropy_norm_uniform() {
        let v = vec![1.0f64; 4];
        let h = shannon_entropy_norm(&v);
        assert!((h - 2.0).abs() < 1e-12, "uniform over 4 ⇒ log2(4)=2 bits");
    }
}
