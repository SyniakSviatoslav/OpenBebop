/**
 * field-rust.ts — WASM bindings to the Rust `bebop-core` graph-PDE core.
 *
 * This is the operator's fix A + B + C applied to the original JS field-sim:
 *   • SPECTRAL PROPAGATOR (field_spectral) — Chebyshev approx of exp(-L·t)·u0 in ONE shot (no K-loop).
 *   • ACTIVE-SET PRUNING  (field_active)   — only |Δu|>ε nodes participate → O(|E_active|) ≪ O(|E|).
 * Rust→WASM gives cache locality + no GC + native f64 (the memory-wall fix D is VSA/SIMD in core).
 *
 * The .wasm is built OFFLINE via `cargo build --release --target wasm32-unknown-unknown` (no network,
 * no external crates). We instantiate it here and marshal CSR + field vectors through linear memory.
 *
 * FLAG-OFF seam: nothing runs until you call a function. Deterministic, no Date/RNG/network.
 */
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const __dirname = dirname(fileURLToPath(import.meta.url));
// Pre-built by cargo (see rust-core/Cargo.toml). Located next to this file after a build.
const WASM_PATH = join(__dirname, '../../rust-core/target/wasm32-unknown-unknown/release/bebop_core.wasm');

// Singleton instance (deterministic, single graph at a time — matches the Rust static scratch).
let _module: WebAssembly.Instance | null = null;

async function getInstance(): Promise<WebAssembly.Instance> {
  if (_module) return _module;
  const bytes = readFileSync(WASM_PATH);
  const mod = await WebAssembly.compile(bytes);
  _module = await WebAssembly.instantiate(mod, {});
  // NOTE: never cache the Memory object's `.buffer` — `memory.grow` detaches it.
  // Always read `inst.exports.memory` fresh (see `liveMem()`).
  return _module;
}

/** Live memory view — re-fetched every call so a mid-run grow never detaches our buffer. */
function liveMem(inst: WebAssembly.Instance): WebAssembly.Memory {
  return inst.exports.memory as WebAssembly.Memory;
}

/** Grow the wasm memory if needed, then return the LIVE buffer (post-grow). */
function ensureMem(inst: WebAssembly.Instance, needBytes: number): ArrayBuffer {
  const mem = liveMem(inst);
  if (mem.buffer.byteLength < needBytes) {
    const pages = Math.ceil((needBytes - mem.buffer.byteLength) / 65536);
    mem.grow(pages); // throws if it exceeds the module's declared max (raised to 64MiB via .cargo config)
  }
  return liveMem(inst).buffer; // re-fetch: grow detaches the old buffer
}

/** Track currently-deployed graph size (set on every build) so size-only calls can't mis-size. */
let _n = 0;

/** Upload a CSR adjacency as f32-packed (halves CSR storage vs `rustBuild`). Compute stays f64. */
export async function rustBuildF32(A: number[][]): Promise<void> {
  const inst = await getInstance();
  const n = A.length;
  _n = n;
  const rowPtr = new Int32Array(n + 1);
  const cols: number[] = [];
  for (let i = 0; i < n; i++) {
    rowPtr[i] = cols.length;
    for (let j = 0; j < n; j++) if (A[i][j]) cols.push(j);
  }
  rowPtr[n] = cols.length;
  const colArr = Float32Array.from(cols); // packed f32
  const rpOff = 0;
  const ciOff = rowPtr.byteLength;
  const need = ciOff + colArr.byteLength;
  const buf = ensureMem(inst, need);
  new Int32Array(buf, rpOff, n + 1).set(rowPtr);
  new Float32Array(buf, ciOff, colArr.length).set(colArr);
  (inst.exports.field_build_f32 as Function)(rpOff, ciOff, colArr.length, n);
}

/**
 * SENSITIVITY BOOTSTRAP (2026-07-09c): per-node criticality derived from the kernel's own
 * accumulated |Δu| history — ZERO new infra (the buffers already exist in the core). Returns
 * Float64Array(n), normalized to [0,1] (most-active node = 1.0). Use this as the `sensitivity`
 * arg to `rustFieldCost`/`rustFieldRank`/`rustFieldArbiter` to make the field weigh nodes by how
 * much they actually move. With no propagations run yet, returns uniform 1.0 (no bias).
 */
export async function rustFieldSensitivity(): Promise<Float64Array> {
  const n = _n;
  if (n === 0) return new Float64Array(0);
  const inst = await getInstance();
  const oOff = 0;
  const need = oOff + n * 8;
  const buf = ensureMem(inst, need);
  const rc = (inst.exports.field_sensitivity as Function)(oOff);
  if (rc !== 0) return new Float64Array(n); // empty graph → neutral
  return Float64Array.from(new Float64Array(liveMem(inst).buffer, oOff, n));
}

/** Upload a symmetric adjacency matrix as CSR into the Rust core. Call before propagate*. */
export async function rustBuild(A: number[][]): Promise<void> {
  const inst = await getInstance();
  const n = A.length;
  _n = n;
  const rowPtr = new Int32Array(n + 1);
  const cols: number[] = [];
  for (let i = 0; i < n; i++) {
    rowPtr[i] = cols.length;
    for (let j = 0; j < n; j++) if (A[i][j]) cols.push(j);
  }
  rowPtr[n] = cols.length;
  const colArr = Int32Array.from(cols);

  const rpOff = 0;
  const ciOff = rowPtr.byteLength;
  const need = ciOff + colArr.byteLength;
  const buf = ensureMem(inst, need);
  new Int32Array(buf, rpOff, n + 1).set(rowPtr);
  new Int32Array(buf, ciOff, colArr.length).set(colArr);
  (inst.exports.field_build as Function)(rpOff, ciOff, colArr.length, n);
}

/**
 * SPECTRAL PROPAGATE — one-shot exp(-L·t)·u0 via Chebyshev (operator fix A).
 * Returns the evolved field vector (length n). `t` is the physical time; `coeff` the diffusion rate.
 */
export async function rustSpectral(u0: Float64Array | number[], t: number, coeff = 1.0, deg = 20): Promise<Float64Array> {
  const inst = await getInstance();
  const n = u0.length;
  const uOff = 0;
  const oOff = n * 8;
  const need = oOff + n * 8;
  const buf = ensureMem(inst, need);
  const ua = Float64Array.from(u0);
  new Float64Array(buf, uOff, n).set(ua);
  const rc = (inst.exports.field_spectral as Function)(uOff, t, coeff, deg, oOff) as number;
  if (rc !== 0) throw new Error(`field_spectral error code ${rc} (deg must be ≥1)`);
  return Float64Array.from(new Float64Array(liveMem(inst).buffer, oOff, n));
}

/**
 * ACTIVE-SET PROPAGATE — K explicit diffusion steps, but only nodes with |Δu|>eps stay in the
 * computation (operator fix C). Returns { field, activePermille } where activePermille is the
 * integer proxy for "graph pruned away" (1000 = fully active, lower = more pruning).
 */
export async function rustActive(
  u0: Float64Array | number[], steps: number, opts: { dt?: number; coeff?: number; eps?: number } = {}
): Promise<{ field: Float64Array; activePermille: number }> {
  const inst = await getInstance();
  const n = u0.length;
  const dt = opts.dt ?? 0.05;
  const coeff = opts.coeff ?? 1.0;
  const eps = opts.eps ?? 1e-4;
  const uOff = 0;
  const oOff = n * 8;
  const aOff = oOff + n * 8;
  const need = aOff + 8;
  const buf = ensureMem(inst, need);
  const ua = Float64Array.from(u0);
  new Float64Array(buf, uOff, n).set(ua);
  (inst.exports.field_active as Function)(uOff, steps, dt, coeff, eps, oOff, aOff);
  const active = new Int32Array(liveMem(inst).buffer, aOff, 1)[0];
  return { field: Float64Array.from(new Float64Array(liveMem(inst).buffer, oOff, n)), activePermille: active };
}

/** VSA similarity (operator fix D: SIMD-ready hypervector dot-product in Rust). */
export async function rustVsaSimilarity(a: Float64Array | number[], b: Float64Array | number[]): Promise<number> {
  const inst = await getInstance();
  const n = a.length;
  const aOff = 0;
  const bOff = n * 8;
  const need = bOff + n * 8;
  const buf = ensureMem(inst, need);
  new Float64Array(buf, aOff, n).set(Float64Array.from(a));
  new Float64Array(buf, bOff, n).set(Float64Array.from(b));
  return (inst.exports.vsa_similarity as Function)(aOff, bOff, n) as number;
}

/**
 * DISPOSE — free the stored graph inside the live WASM instance (calls Rust `field_reset`, which
 * drops the CSR/col/degrees Vecs). Call between graphs to reclaim memory without tearing down the
 * whole instance. The contract: every propagate sequence is preceded by a fresh `rustBuild`.
 */
export async function rustDispose(): Promise<void> {
  const inst = await getInstance();
  (inst.exports.field_reset as Function)();
}

/** Current WASM heap size in bytes (live buffer). Used by leak/stability assertions. */
export async function rustMemoryBytes(): Promise<number> {
  const inst = await getInstance();
  return liveMem(inst).buffer.byteLength;
}

/**
 * BRIDGE B — instant predicted impact (cost) of a disruption `seed` under per-node `sensitivity`.
 * = Σ_i field[i]·sensitivity[i] where field = exp(-L·t)·seed (Chebyshev, one shot). This is the
 * numeric cost predicate a PDDL planner consumes. Returns a finite number ≥ 0, or -1 on error.
 */
export async function rustFieldCost(
  seed: Float64Array | number[],
  opts: { sensitivity?: Float64Array | number[]; t?: number; coeff?: number; deg?: number } = {}
): Promise<number> {
  const inst = await getInstance();
  const n = seed.length;
  const t = opts.t ?? 5.0;
  const coeff = opts.coeff ?? 1.0;
  const deg = opts.deg ?? 24;
  const uOff = 0;
  const sOff = n * 8;
  const need = sOff + n * 8;
  const buf = ensureMem(inst, need);
  new Float64Array(buf, uOff, n).set(Float64Array.from(seed));
  let sensPtr = 0;
  if (opts.sensitivity) {
    if (opts.sensitivity.length !== n) throw new Error('rustFieldCost: sensitivity must have length n');
    new Float64Array(buf, sOff, n).set(Float64Array.from(opts.sensitivity));
    sensPtr = sOff;
  }
  return (inst.exports.field_cost as Function)(uOff, sensPtr, t, coeff, deg) as number;
}

/**
 * BRIDGE A — per-node predicted impact vector (ranked downstream exposure of `seed` weighted by
 * `sensitivity`). Returns Float64Array(n). The Top-K entries are the "Top-K Contours" explainability
 * surface: where a disruption at `seed` will actually hurt.
 */
export async function rustFieldRank(
  seed: Float64Array | number[],
  opts: { sensitivity?: Float64Array | number[]; t?: number; coeff?: number; deg?: number } = {}
): Promise<Float64Array> {
  const inst = await getInstance();
  const n = seed.length;
  const t = opts.t ?? 5.0;
  const coeff = opts.coeff ?? 1.0;
  const deg = opts.deg ?? 24;
  const uOff = 0;
  const sOff = n * 8;
  const oOff = sOff + n * 8;
  const need = oOff + n * 8;
  const buf = ensureMem(inst, need);
  new Float64Array(buf, uOff, n).set(Float64Array.from(seed));
  let sensPtr = 0;
  if (opts.sensitivity) {
    if (opts.sensitivity.length !== n) throw new Error('rustFieldRank: sensitivity must have length n');
    new Float64Array(buf, sOff, n).set(Float64Array.from(opts.sensitivity));
    sensPtr = sOff;
  }
  const rc = (inst.exports.field_rank as Function)(uOff, sensPtr, t, coeff, deg, oOff) as number;
  if (rc !== 0) throw new Error(`field_rank error code ${rc} (empty graph?)`);
  return Float64Array.from(new Float64Array(liveMem(inst).buffer, oOff, n));
}

/**
 * THE FINAL ARBITER (field vs PDDL) — single visible policy.
 * Field is the COST SURFACE, PDDL the EXECUTOR. PDDL's proposed action carries a planner-chosen
 * `pddlCost` (its own symbolic estimate of the disruption the action implies). The field computes
 * `fieldCost` (real downstream impact of that same disruption). Conflict rule:
 *   • fieldCost <= pddlCost              → PERMIT (PDDL already accounts for the real impact; field concurs).
 *   • pddlCost < fieldCost <= pddlCost*mismatchRatio → WARN (field exceeds PDDL but inside the
 *     planner's own slack band — permit but surface to the explainability layer / human).
 *   • fieldCost > pddlCost*mismatchRatio → OVERRIDE (field says PDDL massively under-estimated the
 *     physics; the planner "spery`czetsya" with reality). Returns { verdict, fieldCost, pddlCost }
 *     so the explainability layer can show why the field won.
 *
 * `mismatchRatio` = how far PDDL may trail the field before the field wins (the metaplasticity knob;
 * raise it to trust PDDL more, lower it to let physics dominate). `tolerance` is a hard floor below
 * which any fieldCost is always permitted regardless of PDDL (a contract SLA band for trivial impact).
 */
export type ArbiterVerdict = 'permit' | 'warn' | 'override';
export interface ArbiterResult {
  verdict: ArbiterVerdict;
  fieldCost: number;
  pddlCost: number;
  reason: string;
}
export async function rustFieldArbiterCore(
  seed: Float64Array | number[],
  pddlCost: number,
  opts: {
    sensitivity?: Float64Array | number[];
    t?: number;
    coeff?: number;
    deg?: number;
    tolerance?: number; // fieldCost at or below this is always permitted (SLA floor)
    mismatchRatio?: number; // field wins when fieldCost > pddlCost * mismatchRatio
  } = {},
): Promise<ArbiterResult> {
  const tolerance = opts.tolerance ?? 0.0;
  const mismatchRatio = opts.mismatchRatio ?? 1.5;
  const fieldCost = await rustFieldCost(seed, opts);
  if (fieldCost < 0) return { verdict: 'override', fieldCost, pddlCost, reason: 'field: empty graph / error' };
  if (fieldCost <= tolerance || fieldCost <= pddlCost) {
    return { verdict: 'permit', fieldCost, pddlCost, reason: `fieldCost ${fieldCost.toFixed(4)} ≤ pddlCost ${pddlCost.toFixed(4)} (PDDL covers impact)` };
  }
  if (fieldCost <= pddlCost * mismatchRatio) {
    return {
      verdict: 'warn',
      fieldCost,
      pddlCost,
      reason: `field ${fieldCost.toFixed(4)} > PDDL ${pddlCost.toFixed(4)} but within ${mismatchRatio}× band`,
    };
  }
  return {
    verdict: 'override',
    fieldCost,
    pddlCost,
    reason: `field ${fieldCost.toFixed(4)} > PDDL ${pddlCost.toFixed(4)}×${mismatchRatio} → physics overrides planner`,
  };
}

/** Path to the prebuilt WASM (exposed for tests). */
export const RUST_WASM_PATH = WASM_PATH;

/**
 * TOP-K CONTOURS — the explainability surface. Returns the K nodes where a disruption `seed` will
 * hurt most, each with { index, impact }. `impact` is the per-node `field·sensitivity` from
 * `rustFieldRank`. This is what the explainability layer renders so a human can see WHY the field
 * overrode PDDL — and which nodes to protect first.
 */
export interface Contour {
  index: number;
  impact: number;
}
export async function rustTopKContours(
  seed: Float64Array | number[],
  k: number,
  opts: { sensitivity?: Float64Array | number[]; t?: number; coeff?: number; deg?: number } = {},
): Promise<Contour[]> {
  const rank = await rustFieldRank(seed, opts);
  const idx = Array.from(rank.keys());
  idx.sort((a, b) => rank[b] - rank[a]);
  return idx.slice(0, Math.min(k, idx.length)).map((i) => ({ index: i, impact: rank[i] }));
}

/**
 * AUTOMATIC SENSITIVITY: if no explicit `sensitivity` is passed, bootstrap it from the kernel's
 * accumulated |Δu| history (rustFieldSensitivity) — so the arbiter reflects each node's real
 * criticality with ZERO extra calls from the caller. Pass `opts.sensitivity` to override.
 */
async function resolveSensitivity(
  opts: { sensitivity?: Float64Array | number[] } = {},
): Promise<Float64Array | number[] | undefined> {
  if (opts.sensitivity) return opts.sensitivity;
  const s = await rustFieldSensitivity();
  // rustFieldSensitivity returns uniform 1.0 when no history exists — safe to omit (rank==cost path).
  if (s.length === 0) return undefined;
  return s;
}

// Re-wrap rustFieldArbiter to auto-bootstrap sensitivity. The public signature is unchanged; we
// just resolve sensitivity before delegating to the core cost.
const _arbiterCore = rustFieldArbiterCore;
export async function rustFieldArbiter(
  seed: Float64Array | number[],
  pddlCost: number,
  opts: {
    sensitivity?: Float64Array | number[];
    t?: number;
    coeff?: number;
    deg?: number;
    tolerance?: number;
    mismatchRatio?: number;
    autoSensitivity?: boolean; // default true: bootstrap per-node sensitivity from field history
  } = {},
): Promise<ArbiterResult> {
  const sens = opts.autoSensitivity === false ? opts.sensitivity : await resolveSensitivity(opts);
  return _arbiterCore(seed, pddlCost, { ...opts, sensitivity: sens });
}
