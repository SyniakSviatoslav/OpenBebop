# Bebop — Sovereign Node Deployment (3-Phase)

**Date:** 2026-07-08 · **Status:** design + committable artifacts · **Red-line:** no prod deploy touched (parallel tier, default off)

## 0. The principle (from the operator's hydrodynamics analysis)
A stable agent is one where you, the engineer, control three states of the information-flow field
**J** (data packets = requests/telemetry; ρ = queue/RAM on a node):

1. **∇·J = 0** on the main trunk — steady-state: a node processes exactly what arrives; latency predictable.
2. **Divergence valves** before weak nodes — load balancing (one input fanned to N workers) so a single
   orchestrator never converges (∇·J < 0 → queue grows → OOM/drop).
3. **Curl bounded** — feedback loops (act→evaluate→correct) are deterministic and iteration-capped;
   no infinite rota (∇×J > 0 with no exit = CPU burn, zero useful work).

The Sovereign Node is the *physical* realization: client-owned hardware, no external egress, the
deterministic core (bebop_core.wasm) is the trunk kept at ∇·J = 0; Ollama (local LLM) is the
divergence valve; the guard/recall loops are the bounded curl.

## 1. What bebop already has (grounding — not invented)
- `crates/core/build.sh` → builds `bebop_core.wasm` to `wasm32-unknown-unknown` (REAL, 183 KB, verified this session).
- `src/core-wasm.ts` → `initCore()` loads that wasm in-process; exports `decide/embed/similarity/estimateTokens/exportLog`.
- `src/guard.ts` delegates to the core; `free-llm.ts` + `llm-manifest.json` are the LLM periphery.
- No existing Dockerfile/compose → Phases 1-3 are net-new, parallel, default-off.

## 2. Three phases (EV / risk / when)

| Phase | Packaging | Deterministic core | LLM periphery | EV(+) | Risk(−) | Pick when |
|---|---|---|---|---|---|---|
| **1. Offline OCI** | `Dockerfile.sovereign` + `docker-compose.sovereign.yml` (OCI tarball / USB) | wasm32 in-process loader | Ollama sidecar, local, internal net | fastest B2B path; uses existing tooling; CPU works | image 5-10GB; needs Docker on client; root daemon | default for any client box (CPU or GPU) |
| **2. WASI/WasmEdge** | `crates/core/build-wasi.sh` + `src/core-wasi.ts` + `wasmedge` run | **WasmEdge AOT** (MB, sub-ms, proven sandbox) | Ollama native (WASI-NN optional later) | hardened core, tamper-evident, bit-identical; cheapest-token principle | needs wasmedge on box; core-only hardening (LLM still native) | client wants hardened core isolation w/o unikernel cost |
| **3. Unikernel** | `docs/design/bebop-sovereign-node-UNIKERNEL-2026-07-08.md` (OPS/Nanos) | wasm32 core in-image | Ollama unikernel sidecar (GPU klib) IF client has GPU | zero OS surface, no shell/SSH/exfil; single-tenant (beats P2P-swarm leaks/Sybil) | bleeding-edge; needs hypervisor + real GPU; setup cost high; does NOT shrink model weights/CUDA wall | client owns RTX + demands max isolation + runs a hypervisor |

**Honest limits (per operator's own swarm critique):** unikernels don't shrink GGUF weights or remove
the CUDA-driver hardware wall; a dusty Intel-UHD/4GB client can't run Phase 3. The "dusty office PC"
client gets Phase 1 (CPU Ollama). Phase 3 is the *narrowest* market, kept optional.

## 3. How the tiers compose (no core re-write)
- The deterministic core is ONE artifact family (`bebop_core.wasm` for phases 1&3-in-process,
  `bebop_core.wasi.wasm` for phase 2). Same Rust crate, same wasm32-clean invariant.
- `BEBOP_CORE_RUNTIME` selects the loader: `inproc` (default, `core-wasm.ts`) vs `wasi`
  (`core-wasi.ts`, WasmEdge). Both expose the identical `CoreHandle` interface → `guard.ts` unchanged.
- `BEBOP_ALLOW_EXTERNAL_LLM=0` (set in all sovereign artifacts) is the egress kill-switch: the shell
  refuses any non-local LLM base URL. This enforces ∇·J = 0 for the money/data path (no external leak).
- `BEBOP_TELEMETRY_SINK=local` keeps telemetry on-box (no external push under sanctions/blockade).

## 4. Artifacts produced this session
- `Dockerfile.sovereign` — Phase 1 runtime (core-builder + node runtime + Ollama sidecar comments).
- `docker-compose.sovereign.yml` — Phase 1 bring-up, `internal: true` network, air-gap `docker save` recipe.
- `crates/core/build-wasi.sh` — Phase 2 wasm32-wasi build + WasmEdge AOT.
- `src/core-wasi.ts` — Phase 2 WASI loader (`initWasiCore`), same `CoreHandle` contract.
- `docs/design/bebop-sovereign-node-UNIKERNEL-2026-07-08.md` — Phase 3 runbook + config.
- `src/core-wasi.test.ts` — RED+GREEN: selector returns null gracefully when wasmedge/binary absent.

## 5. Verification
- `crates/core/build.sh` → real wasm32 build (183 KB) PASS.
- `node --test --import tsx src/*.test.ts` → 223 pass / 0 fail (regression: new loader is additive).
- `npx tsc --noEmit` → exit 0 (authoritative; sandbox default tsc flags are broken for this repo).
- Phase 2/3 runtime binaries (wasmedge / ops) are NOT installed here → those paths are spec'd + the
  loader degrades to null (proven by test), so the box still boots on the in-process core.

## 6. Next (needs operator go)
- Wire `BEBOP_CORE_RUNTIME` into `guard.ts` init (one-line selector) — currently additive, uncommitted.
- Add `docker build -f Dockerfile.sovereign` to CI as a non-blocking job (mirrors proposed-sovereign-core-ci).
- Ship `bebop_core.wasi.wasm` into the dist/ artifact set.
