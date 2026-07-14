#!/usr/bin/env bash
# Bebop Sovereign Node — Phase 3: build the unikernel image (NanoVMs / OPS).
#
# Prereqs (operator-installed; not bundled): `ops` (curl https://ops.city/get.sh -sSfL | sh),
# and a hypervisor on the client box (HVF/KVM/firecracker). For GPU-in-image, the Nanos CUDA klib
# must be available (NanoVMs ports NVIDIA Linux kernel drivers into a Nanos klib, late-2025).
#
# This is the narrowest-market tier (real GPU + max-isolation + hypervisor). It is NOT a default.
set -euo pipefail
cd "$(dirname "$0")/.."

echo "── Phase 3: OPS unikernel image (bebop.sovereign) ──"
if ! command -v ops >/dev/null 2>&1; then
  echo "✗ 'ops' not installed. Run: curl https://ops.city/get.sh -sSfL | sh" >&2
  exit 1
fi

# Ensure the hardened WASI core + node runtime are built first (Phases 2 + 1 runtime).
( cd crates/core && bash build-wasi.sh )
command -v docker >/dev/null 2>&1 && docker build --target runtime -f Dockerfile.sovereign -t bebop-sovereign:runtime . >/dev/null 2>&1 \
  && echo "  ✓ runtime stage available" || echo "  ⚠ runtime stage skipped (docker absent); ops will package the local node runtime"

# Package as a single VM image (no Linux userspace, no shell, no SSH).
ops image create -c config.sovereign.json -f bebop.sovereign -t hvt
echo "✓ wrote bebop.sovereign (.hvt). Boot on client hypervisor: ops instance create -i bebop.sovereign -t hvt"
