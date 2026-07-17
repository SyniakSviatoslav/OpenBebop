//! B4 — durable ledger recorder.
//!
//! Runs the §2.1 benches for REAL on the live deployment host and appends one
//! append-only JSONL row per bench to `docs/ledger/crypto-bench.jsonl` (never
//! overwrites), plus a human-readable `bench/BENCH_RESULTS.md`. Host/CPU are read
//! from the LIVE machine at run time (never a literal, per the DoD); commits are
//! captured via `git rev-parse`. Zero external deps — a self-contained
//! `std::time::Instant` percentile sampler, because the ledger schema's `p99_ns`
//! field is not exposed by criterion's estimates output.
//!
//! Run:  cargo run --offline -p bebop2-bench --bin record-ledger

use std::fs;
use std::hint::black_box;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use bebop2_bench as b;
use bebop2_core::{hash, pq_dsa, sign};

// Sampler configuration (stable — feeds run_key).
const WARMUP_S: f64 = 3.0;
const MEASURE_S: f64 = 5.0;
const SAMPLE_CAP: usize = 100_000;

fn read_first_line(path: &str) -> Option<String> {
    fs::read_to_string(path)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Live hostname from the kernel (never a literal — DoD item 1).
fn live_host() -> String {
    read_first_line("/proc/sys/kernel/hostname")
        .or_else(|| std::env::var("HOSTNAME").ok())
        .unwrap_or_else(|| "unknown-host".to_string())
}

/// Live CPU model string from /proc/cpuinfo (never a literal — DoD item 1).
fn live_cpu() -> String {
    fs::read_to_string("/proc/cpuinfo")
        .ok()
        .and_then(|s| {
            s.lines()
                .find(|l| l.starts_with("model name"))
                .and_then(|l| l.split(':').nth(1))
                .map(|v| v.trim().to_string())
        })
        .unwrap_or_else(|| "unknown-cpu".to_string())
}

fn git_head(dir: &str) -> String {
    let head = Command::new("git")
        .args(["-C", dir, "rev-parse", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "unavailable".to_string());
    // Provenance honesty: a run on a dirty tree measures code that is NOT the HEAD
    // commit. Stamp it visibly so a dirty-tree row can never masquerade as a
    // clean-commit measurement (the suffix also feeds run_key, forcing a distinct
    // identity). A clean re-run on the landed commit is the durable citation.
    let dirty = Command::new("git")
        .args(["-C", dir, "status", "--porcelain"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| !o.stdout.is_empty())
        .unwrap_or(false);
    if dirty { format!("{head}+dirty") } else { head }
}

fn json_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

struct Ctx {
    host: String,
    cpu: String,
    commit_bebop: String,
    commit_dowiz: String,
}

#[allow(clippy::too_many_arguments)]
fn row_json(
    ctx: &Ctx,
    bench_id: &str,
    st: &b::Stats,
    msg_bytes: usize,
    chain_depth: i64,
    ts: u64,
) -> String {
    let key = b::run_key(
        bench_id,
        &ctx.commit_bebop,
        &ctx.commit_dowiz,
        &ctx.host,
        &ctx.cpu,
        msg_bytes,
        chain_depth,
        SAMPLE_CAP,
        WARMUP_S,
        MEASURE_S,
    );
    format!(
        "{{\"ts\":{ts},\"host\":\"{host}\",\"cpu\":\"{cpu}\",\"commit_bebop\":\"{cb}\",\
         \"commit_dowiz\":\"{cd}\",\"run_key\":\"{key}\",\"bench_id\":\"{bench}\",\
         \"mean_ns\":{mean:.1},\"median_ns\":{median},\"ci95_low_ns\":{lo:.1},\
         \"ci95_high_ns\":{hi:.1},\"p99_ns\":{p99},\"min_ns\":{min},\"samples\":{samples},\
         \"samples_cfg\":{cap},\"warmup_s\":{warm:.1},\"measure_s\":{meas:.1},\
         \"msg_bytes\":{msg_bytes},\"chain_depth\":{chain_depth}}}",
        host = json_escape(&ctx.host),
        cpu = json_escape(&ctx.cpu),
        cb = json_escape(&ctx.commit_bebop),
        cd = json_escape(&ctx.commit_dowiz),
        bench = json_escape(bench_id),
        mean = st.mean_ns,
        median = st.median_ns,
        lo = st.ci95_low_ns,
        hi = st.ci95_high_ns,
        p99 = st.p99_ns,
        min = st.min_ns,
        samples = st.samples,
        cap = SAMPLE_CAP,
        warm = WARMUP_S,
        meas = MEASURE_S,
    )
}

fn measure<F: FnMut()>(f: F) -> b::Stats {
    let raw = b::sample(f, WARMUP_S, MEASURE_S, SAMPLE_CAP);
    b::stats_from(&raw)
}

/// Borrowed batch-verify view over the first `n` owned fixtures.
fn view(src: &[([u8; 32], Vec<u8>, [u8; 64])], n: usize) -> Vec<(&[u8; 32], &[u8], &[u8; 64])> {
    src[..n].iter().map(|(pk, m, s)| (pk, m.as_slice(), s)).collect()
}

fn main() {
    // Repo root = <manifest>/../.. (bebop2/bench -> repo root).
    let repo_root: PathBuf = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("resolve repo root");
    let ledger_dir = repo_root.join("docs/ledger");
    let ledger_path = ledger_dir.join("crypto-bench.jsonl");
    let results_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("BENCH_RESULTS.md");

    let ctx = Ctx {
        host: live_host(),
        cpu: live_cpu(),
        commit_bebop: git_head(repo_root.to_str().unwrap()),
        commit_dowiz: git_head("/root/dowiz-agentic-mesh"),
    };

    eprintln!(
        "B4 ground-truth bench — host={} cpu={} commit_bebop={} commit_dowiz={}",
        ctx.host, ctx.cpu, ctx.commit_bebop, ctx.commit_dowiz
    );
    eprintln!("config: warmup {WARMUP_S}s, measure {MEASURE_S}s, sample cap {SAMPLE_CAP}\n");

    // ── Build fixtures once (outside timing) ──────────────────────────────────
    let (ed_pk, ed_msg, ed_sig) = b::ed25519_single_fixture();
    let (mldsa_pk_b, mldsa_msg, mldsa_sig_b) = b::mldsa_frame_fixture();
    let (mldsa_lg_pk_b, mldsa_lg_msg, mldsa_lg_sig_b) = b::mldsa_large_fixture(3400);
    let (f1, roster1, chain1) = b::build_frame(1, b::PAYLOAD_BYTES, [11u8; 8]);
    let revs_empty = bebop_proto_cap::revocation::RevocationSet::new();
    let (f3, roster3, chain3) = b::build_frame(3, b::PAYLOAD_BYTES, [13u8; 8]);
    let revs_10k = b::build_revocations(10_000);
    let sha_input = vec![0xC3u8; 1024];

    // Batch fixtures (owned); borrowed views built via the `view` helper.
    let batch64 = b::ed25519_batch_fixtures(64);

    let mldsa_pk = pq_dsa::MlDsa65Pk { bytes: mldsa_pk_b.clone() };
    let mldsa_sig = pq_dsa::MlDsa65Sig { bytes: mldsa_sig_b.clone() };
    let mldsa_lg_pk = pq_dsa::MlDsa65Pk { bytes: mldsa_lg_pk_b.clone() };
    let mldsa_lg_sig = pq_dsa::MlDsa65Sig { bytes: mldsa_lg_sig_b.clone() };

    // Sanity: every fixture verifies before we bench it.
    assert!(sign::verify(&ed_pk, &ed_msg, &ed_sig), "ed25519 fixture invalid");
    assert!(pq_dsa::verify(&mldsa_pk, &mldsa_msg, &mldsa_sig), "mldsa fixture invalid");
    assert!(pq_dsa::verify(&mldsa_lg_pk, &mldsa_lg_msg, &mldsa_lg_sig), "mldsa-large fixture invalid");
    b::gate_check_once(&f1, &roster1, &chain1, &revs_empty);
    b::gate_check_once(&f3, &roster3, &chain3, &revs_10k);
    {
        let v = view(&batch64, 64);
        assert!(sign::verify_batch(&v), "batch64 fixture invalid");
    }

    // ── Benches: (bench_id, msg_bytes, chain_depth, Stats) ────────────────────
    let mut rows: Vec<(String, usize, i64, b::Stats)> = Vec::new();

    eprintln!("[1/8] ed25519_verify_single ...");
    let st = measure(|| {
        black_box(sign::verify(black_box(&ed_pk), black_box(&ed_msg), black_box(&ed_sig)));
    });
    rows.push(("ed25519_verify_single".into(), ed_msg.len(), 0, st));

    eprintln!("[2/8] mldsa65_verify_single ...");
    let st = measure(|| {
        black_box(pq_dsa::verify(black_box(&mldsa_pk), black_box(&mldsa_msg), black_box(&mldsa_sig)));
    });
    rows.push(("mldsa65_verify_single".into(), mldsa_msg.len(), 0, st));

    eprintln!("[3/8] mldsa65_verify_single_3400 ...");
    let st = measure(|| {
        black_box(pq_dsa::verify(black_box(&mldsa_lg_pk), black_box(&mldsa_lg_msg), black_box(&mldsa_lg_sig)));
    });
    rows.push(("mldsa65_verify_single_3400".into(), mldsa_lg_msg.len(), 0, st));

    eprintln!("[4/8] hybrid_gate_check/d1 ...");
    let gate_msg_bytes = f1.binding_signing_domain().unwrap().len();
    let st = measure(|| {
        b::gate_check_once(black_box(&f1), black_box(&roster1), black_box(&chain1), black_box(&revs_empty));
    });
    rows.push(("hybrid_gate_check/d1".into(), gate_msg_bytes, 1, st));

    eprintln!("[5/8] hybrid_gate_check/d3_rev10k ...");
    let st = measure(|| {
        b::gate_check_once(black_box(&f3), black_box(&roster3), black_box(&chain3), black_box(&revs_10k));
    });
    rows.push(("hybrid_gate_check/d3_rev10k".into(), gate_msg_bytes, 3, st));

    eprintln!("[6/8] sha3_256_1kib ...");
    let st = measure(|| {
        black_box(hash::sha3_256(black_box(&sha_input)));
    });
    rows.push(("sha3_256_1kib".into(), 1024, 0, st));

    eprintln!("[7/8] ed25519_verify_batch/8 ...");
    let st = measure(|| {
        let v = view(&batch64, 8);
        black_box(sign::verify_batch(black_box(&v)));
    });
    rows.push(("ed25519_verify_batch/8".into(), batch64[0].1.len(), 0, st));

    eprintln!("[8/8] ed25519_verify_batch/64 ...");
    let st = measure(|| {
        let v = view(&batch64, 64);
        black_box(sign::verify_batch(black_box(&v)));
    });
    rows.push(("ed25519_verify_batch/64".into(), batch64[0].1.len(), 0, st));

    // ── Append to the ledger (append-only, never overwrite) ───────────────────
    fs::create_dir_all(&ledger_dir).expect("create docs/ledger");
    let ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    let mut jsonl = String::new();
    for (bench_id, msg_bytes, depth, st) in &rows {
        jsonl.push_str(&row_json(&ctx, bench_id, st, *msg_bytes, *depth, ts));
        jsonl.push('\n');
    }
    let mut fh = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&ledger_path)
        .expect("open ledger for append");
    fh.write_all(jsonl.as_bytes()).expect("append ledger rows");

    // ── Human-readable capture (BENCH_RESULTS.md convention) ──────────────────
    let mut md = String::new();
    md.push_str("# B4 — crypto ground-truth BENCH_RESULTS\n\n");
    md.push_str(&format!(
        "- host: `{}`\n- cpu: `{}`\n- commit_bebop: `{}`\n- commit_dowiz: `{}`\n- config: warmup {}s, measure {}s, sample cap {}\n- harness: zero-dep `std::time::Instant` percentile sampler (p99 not exposed by criterion)\n- caveats: single-threaded (gate mutex contention understated); for sub-µs ops the Instant call-pair overhead (~tens of ns) is a few %% (sha3 anchor).\n\n",
        ctx.host, ctx.cpu, ctx.commit_bebop, ctx.commit_dowiz, WARMUP_S, MEASURE_S, SAMPLE_CAP
    ));
    md.push_str("| bench_id | mean | median | p99 | min | samples | msg_bytes | depth |\n");
    md.push_str("|---|---:|---:|---:|---:|---:|---:|---:|\n");
    let fmt_ns = |ns: f64| -> String {
        if ns >= 1_000_000.0 {
            format!("{:.3} ms", ns / 1_000_000.0)
        } else if ns >= 1_000.0 {
            format!("{:.2} µs", ns / 1_000.0)
        } else {
            format!("{ns:.0} ns")
        }
    };
    for (bench_id, msg_bytes, depth, st) in &rows {
        md.push_str(&format!(
            "| `{}` | {} | {} | {} | {} | {} | {} | {} |\n",
            bench_id,
            fmt_ns(st.mean_ns),
            fmt_ns(st.median_ns as f64),
            fmt_ns(st.p99_ns as f64),
            fmt_ns(st.min_ns as f64),
            st.samples,
            msg_bytes,
            depth
        ));
    }
    // Batch-vs-singles honest note.
    let single_mean = rows.iter().find(|r| r.0 == "ed25519_verify_single").map(|r| r.3.mean_ns);
    let batch64_mean = rows.iter().find(|r| r.0 == "ed25519_verify_batch/64").map(|r| r.3.mean_ns);
    if let (Some(s), Some(b64)) = (single_mean, batch64_mean) {
        md.push_str(&format!(
            "\n**Batch vs singles (Ed25519):** 64 × single ≈ {} vs batch/64 = {} → batch costs \
             {:.2}× the 64 singles (SLOWER, by design — not a regression). The F1 soundness fix \
             (SSR-2020 mixed-order forgery class; `sign.rs::verify_batch`, 2026-07-17) confirms \
             EVERY batch-accept with a full per-item cofactorless single verify, so the accept \
             path always costs the batch equation PLUS N singles — ≥ N singles regardless of \
             scalar-mul optimization. The batch equation is a sound fast-REJECT / accept-HINT \
             only; batching currently has NO throughput benefit. A Straus/Pippenger multi-scalar \
             mult (out of scope, DECART-gated per B4 §5) would cheapen only the hint/reject leg — \
             the N confirming singles remain. Correctness over throughput, recorded honestly.\n",
            fmt_ns(s * 64.0),
            fmt_ns(b64),
            b64 / (s * 64.0)
        ));
    }
    md.push_str("\n**Envelope tax (recomputed):** SIGNATUREBYTES=3309 + Ed25519 64 = 3373 raw; + 2×(1+4) TLV framing = **3383 B ≈ 3.3 KiB** (R4's ~3.4 KB confirmed). PQ pubkey 1952 B is referenced by 32-byte `pq_key_id`, never re-shipped per frame.\n");
    fs::write(&results_path, md).expect("write BENCH_RESULTS.md");

    eprintln!("\nappended {} rows -> {}", rows.len(), ledger_path.display());
    eprintln!("wrote {}", results_path.display());
    // Echo the rows to stdout so the run is self-documenting.
    println!("{jsonl}");
}
