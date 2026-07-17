# B4 — crypto ground-truth BENCH_RESULTS

- host: `dowiz-dev`
- cpu: `AMD EPYC-Milan Processor`
- commit_bebop: `397b8cd8bcd530a9690e77e60d7ffd9518dc170d+dirty`
- commit_dowiz: `f3018926223e9a2dcdd0f522c9b2a81763776f6d`
- config: warmup 3s, measure 5s, sample cap 100000
- harness: zero-dep `std::time::Instant` percentile sampler (p99 not exposed by criterion)
- caveats: single-threaded (gate mutex contention understated); for sub-µs ops the Instant call-pair overhead (~tens of ns) is a few %% (sha3 anchor).

| bench_id | mean | median | p99 | min | samples | msg_bytes | depth |
|---|---:|---:|---:|---:|---:|---:|---:|
| `ed25519_verify_single` | 629.56 µs | 624.25 µs | 771.92 µs | 614.39 µs | 7941 | 402 | 0 |
| `mldsa65_verify_single` | 791.56 µs | 780.09 µs | 896.80 µs | 771.17 µs | 6317 | 402 | 0 |
| `mldsa65_verify_single_3400` | 820.73 µs | 818.33 µs | 895.61 µs | 809.33 µs | 6092 | 3400 | 0 |
| `hybrid_gate_check/d1` | 2.072 ms | 2.059 ms | 2.315 ms | 2.044 ms | 2414 | 402 | 1 |
| `hybrid_gate_check/d3_rev10k` | 3.423 ms | 3.378 ms | 4.285 ms | 3.347 ms | 1461 | 402 | 3 |
| `sha3_256_1kib` | 2.54 µs | 2.50 µs | 3.44 µs | 2.46 µs | 100000 | 1024 | 0 |
| `ed25519_verify_batch/8` | 16.528 ms | 16.445 ms | 17.230 ms | 16.234 ms | 303 | 34 | 0 |
| `ed25519_verify_batch/64` | 131.167 ms | 130.208 ms | 147.108 ms | 128.767 ms | 39 | 34 | 0 |

**Batch vs singles (Ed25519):** 64 × single ≈ 40.292 ms vs batch/64 = 131.167 ms → batch costs 3.26× the 64 singles (SLOWER, by design — not a regression). The F1 soundness fix (SSR-2020 mixed-order forgery class; `sign.rs::verify_batch`, 2026-07-17) confirms EVERY batch-accept with a full per-item cofactorless single verify, so the accept path always costs the batch equation PLUS N singles — ≥ N singles regardless of scalar-mul optimization. The batch equation is a sound fast-REJECT / accept-HINT only; batching currently has NO throughput benefit. A Straus/Pippenger multi-scalar mult (out of scope, DECART-gated per B4 §5) would cheapen only the hint/reject leg — the N confirming singles remain. Correctness over throughput, recorded honestly.

**Envelope tax (recomputed):** SIGNATUREBYTES=3309 + Ed25519 64 = 3373 raw; + 2×(1+4) TLV framing = **3383 B ≈ 3.3 KiB** (R4's ~3.4 KB confirmed). PQ pubkey 1952 B is referenced by 32-byte `pq_key_id`, never re-shipped per frame.

## Provenance note for THIS run (2026-07-17, pre-commit re-bench)

`commit_bebop` reads `397b8cd8…+dirty` because the run was performed on the working tree
**containing the staged F1-fixed `verify_batch`** (SSR-2020 mixed-order hardening) atop parent
commit `397b8cd8` — the fix was not yet committed at bench time, so no clean commit hash exists
that contains the measured code. These rows replace an earlier 2026-07-17 run that (a) cited
`397b8cd8` cleanly although `verify_batch` does not exist at that commit, and (b) measured the
PRE-fix batch path (batch/8 10.898 ms, batch/64 85.388 ms — ~1.5× faster than the code actually
shipping, because it lacked the per-item confirming verifies). Non-batch rows are statistically
unchanged (within ~0.5%), confirming the fix touched only the batch path. Per B4 §3 step 6, a
clean re-run of `record-ledger` on the landed commit is the follow-up that mints the durable
`ledger:<id>.<stat>` citations; until then, dirty-stamped rows deliberately resolve to no clean
run_key.
