# Bebop Sovereign Node — Phase 3: Unikernel (NanoVMs / OPS)
#
# Operator directive (2026-07-08): for the client with REAL GPU + max-isolation requirement, package
# bebop as a single VM image with no shell, no SSH, no exfil path. This is the "fortress" tier.
#
# STATUS: runbook + image spec (NOT a default). Phase 1 (OCI) is the shipped default; Phase 2
# (WASI/WasmEdge) is the hardened-core option; Phase 3 is the narrowest-market, highest-setup-cost
# tier — only when the client has a GPU and demands no-OS-surface isolation.
#
# Reality check (applied honestly, per the operator's own critique of P2P swarms):
#   - Unikernels do NOT shrink model weights or remove the CUDA-driver hardware wall. A dusty office
#     PC with Intel UHD + 4GB RAM still cannot run this. GPU passthrough needs the client's exact
#     driver + a real GPU. NanoVMs ports NVIDIA Linux kernel drivers into a Nanos klib (late-2025),
#     so GPU-in-unikernel IS possible now — but it is single-tenant, single-app, bleeding-edge.
#   - The reason unikernels WIN here (vs the P2P swarm the operator rejected): single-tenant isolated
#     box = no cross-tenant memory leakage, no Sybil poisoning surface, you control node lifetime.
#     They are the PACKAGING, not the architecture. The architecture is the same dedicated Sovereign
#     Node from Phases 1-2.

## 1. What goes in the image
- The bebop TS runtime (Phase 1 runtime stage) OR the WASI core (Phase 2) as the decision module.
- A slim Ollama unikernel sidecar (Nanos klib) ONLY if the client has GPU + wants in-image inference.
  CPU-only clients keep Ollama as a separate native process on the box (simpler, same isolation win).
- bebop_core.wasm / bebop_core.wasi.wasm as the deterministic kernel (bit-identical to prod core).

## 2. Build (OPS / Nanos)
```bash
# Install OPS (NanoVMs unikernel builder)
curl https://ops.city/get.sh -sSfL | sh
export PATH="$HOME/.ops/bin:$PATH"

# Package the bebop runtime as a unikernel image (no Linux userspace, no shell)
ops image create -c config.sovereign.json -f bebop.sovereign -t hvt

# (Optional GPU) use the Nanos CUDA klib so the image can drive the client's RTX:
#   ops image create -c config.sovereign.json -f bebop.sovereign -t hvt --kvm-cuda
```

## 3. Runtime config (config.sovereign.json)
```json
{
  "Program": "/app/bebop.ts",
  "ProgramEnv": [
    "BEBOP_SOVEREIGN=1",
    "BEBOP_CORE_PATH=/app/src/bebop_core.wasm",
    "BEBOP_LLM_BASE_URL=http://127.0.0.1:11434",
    "BEBOP_ALLOW_EXTERNAL_LLM=0",
    "BEBOP_TELEMETRY_SINK=local"
  ],
  "Files": ["/app/src/bebop_core.wasm"],
  "NoSSHEcho": true,
  "DumpOnExit": false
}
```

## 4. Deploy on the client box
```bash
# Boot the unikernel directly on their hypervisor (HVF/KVM/firecracker)
ops instance create -i bebop.sovereign -t hvt
# Or hand them a pre-built .hvt image on air-gapped USB; they boot it on their hypervisor.
```
No container daemon, no root container socket, no SSH — the attack surface is the single app + its
network listener. If an attacker breaches the box there is no shell to pivot through.

## 5. When to pick this tier (EV/risk)
- PICK: client owns RTX-class GPU, regulatory/paranoia requirement for zero-OS-surface, willing to
  run a hypervisor. Highest isolation, smallest attack surface, deterministic core intact.
- SKIP: CPU-only client (use Phase 1 OCI), or no hypervisor maturity (use Phase 2 WASI on plain
  Docker). Unikernel tooling is still young; budget setup time accordingly.
