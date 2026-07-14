#!/usr/bin/env bash
# CI GUARD — KERNEL COMPILE-FENCE (MESH-02).
# The facade (proto-cap) is the ONLY boundary to the host kernel. The kernel's
# money/decide/fold semantics must NEVER be reachable from proto-cap's own
# dependency graph. So: proto-cap MUST NOT depend on dowiz-kernel.
set -euo pipefail
cd "$(dirname "$0")/.."
if grep -qE 'dowiz-kernel|dowiz_kernel' bebop2/proto-cap/Cargo.toml; then
  echo "KERNEL-FENCE violation: proto-cap/Cargo.toml must NOT depend on dowiz-kernel"; exit 1
fi
echo "PASS: KERNEL COMPILE-FENCE — proto-cap does not depend on dowiz-kernel."
