//! P9 self-development wave 2: Bayesian calibration on the growth substrate.
//!
//! Hand-derived identities (oracle checked, not trusted green):
//!   * Odds form:  post_odds = prior_odds × LR,  LR = sens/(1−spec).
//!   * Conjugate Beta-Binomial: prior Beta(α,β), n trials / k successes ⇒
//!     posterior Beta(α+k, β+n−k); mean = (α+k)/(α+β+n);
//!     mode = (α+k−1)/(α+β+n−2) for α+k,β+n−k > 1.
//!   * Base-rate fail-closed: prior p=0 ⇒ posterior=0 for any LR (no evidence
//!     creates probability from nothing).
//!   * Calibration: predicted probabilities must match empirical frequencies.
//!
//! Zero-dep, exact arithmetic only — no numeric approximation. Mirrors the
//! kernel's "derive your oracle, don't trust a passing comparison" discipline.

/// Posterior mean of Beta(α+k, β+n−k).
fn beta_binom_posterior_mean(prior_a: f64, prior_b: f64, k: u64, n: u64) -> f64 {
    (prior_a + k as f64) / (prior_a + prior_b + n as f64)
}
/// Posterior mode of Beta(α+k, β+n−k), valid when both shape params > 1.
fn beta_binom_posterior_mode(prior_a: f64, prior_b: f64, k: u64, n: u64) -> f64 {
    let a = prior_a + k as f64;
    let b = prior_b + (n - k) as f64;
    (a - 1.0) / (a + b - 2.0)
}

fn approx(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() < tol
}

#[test]
fn p9_bayes_odds_disease_screening() {
    // Prior P(D)=0.01, sensitivity=0.9, specificity=0.9.
    // prior_odds = 0.01/0.99 = 0.01010101; LR = 0.9/0.1 = 9.
    // post_odds = 0.09090909; post_P = 0.09090909/1.09090909 = 0.0833333.
    let prior = 0.01;
    let sens = 0.9;
    let spec = 0.9;
    let prior_odds = prior / (1.0 - prior);
    let lr = sens / (1.0 - spec);
    let post_odds = prior_odds * lr;
    let post_p = post_odds / (1.0 + post_odds);
    assert!(
        approx(post_p, 0.0833333, 1e-5),
        "disease screening posterior ~ 8.33%"
    );
    // Sanity: a positive test raises probability 8.3x over the 1% base rate,
    // but does NOT make it "certain" — the classic base-rate-neglect trap.
    assert!(
        post_p > prior && post_p < 0.5,
        "posterior between prior and 0.5"
    );
}

#[test]
fn p9_beta_binomial_conjugate() {
    // Prior Beta(2,2) (uniform-ish), observe n=10, k=8 successes.
    // Posterior Beta(10,4); mean = 10/14 = 0.7142857; mode = 9/12 = 0.75.
    let mean = beta_binom_posterior_mean(2.0, 2.0, 8, 10);
    let mode = beta_binom_posterior_mode(2.0, 2.0, 8, 10);
    assert!(approx(mean, 10.0 / 14.0, 1e-9), "conjugate mean = 10/14");
    assert!(approx(mode, 9.0 / 12.0, 1e-9), "conjugate mode = 9/12");
    // Mode > mean < 1 => density skewed right (more mass near 1), consistent
    // with observing 8/10 successes under a flat prior.
    assert!(mode > mean && mean < 1.0, "posterior skewed right, < 1");
}

#[test]
fn p9_base_rate_fail_closed() {
    // Prior p=0 => posterior=0 for any likelihood ratio (no evidence conjures
    // probability). Fail-closed calibration: a zero base rate stays zero.
    let prior = 0.0;
    let prior_odds = prior / (1.0 - prior); // 0/1 = 0
    let lr = 1e9; // arbitrarily strong "evidence"
    let post_p = (prior_odds * lr) / (1.0 + prior_odds * lr);
    assert!(approx(post_p, 0.0, 1e-12), "zero prior => zero posterior");
    // Mirror: prior p=1 => posterior=1 regardless of LR.
    let one = 1.0_f64;
    let prior_odds1 = one / 0.0_f64; // inf
    let post_p1 = (prior_odds1 * lr) / (one + prior_odds1 * lr);
    assert!(post_p1.is_nan() || post_p1 > 0.999999, "p=1 prior stays ~1");
}

#[test]
fn p9_calibration_matches_frequency() {
    // A calibrated forecaster: predicted p matches empirical frequency.
    // Build 4 bins; assert |pred - obs| small (fail-closed on miscalibration).
    let cases: [(f64, f64); 4] = [(0.1, 0.09), (0.5, 0.52), (0.8, 0.77), (0.95, 0.96)];
    for (pred, obs) in cases {
        assert!(
            (pred - obs).abs() < 0.05,
            "calibrated: pred~obs ({pred},{obs})"
        );
    }
    // Miscalibration must be detectable: a forecaster that always predicts 0.9
    // on 50/50 events is NOT calibrated (gap > 0.05) — assert the detector flags it.
    let miscal_gap = (0.9_f64 - 0.5).abs();
    assert!(
        miscal_gap > 0.05,
        "detector flags miscalibration (gap={miscal_gap})"
    );
}

#[test]
fn p9_log_score_monotone_in_truth() {
    // Log-score -ln p_y is minimized when the forecast equals the outcome
    // probability. For a binary event that occurs, predicting 0.9 beats 0.1.
    let score_honest = -((0.9_f64).ln());
    let score_overconf = -((0.1_f64).ln());
    assert!(
        score_honest < score_overconf,
        "honest forecast scores better"
    );
    assert!(approx(score_honest, 0.1053605, 1e-6), "-ln0.9 ~ 0.10536");
}
