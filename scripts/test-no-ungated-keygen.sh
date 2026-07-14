#!/usr/bin/env bash
# RED+GREEN regression for the C3 guard.
set -euo pipefail
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
GUARD="$REPO_ROOT/scripts/ci-no-ungated-keygen.sh"

echo "== GREEN: real repo has the C3 gate =="
bash "$GUARD" && echo "GREEN PASS" || { echo "GREEN FAIL"; exit 1; }

echo "== RED: a build with the gate removed must trip the guard =="
TMP="$(mktemp -d)"
mkdir -p "$TMP/bebop2/core/src"
# Fabricate a minimal tree: a pq_dsa.rs whose keygen is UNGATED.
cat > "$TMP/bebop2/core/src/pq_dsa.rs" <<'EOF'
// ungated on purpose — simulates C3 regression
pub fn keygen(seed: &[u8; 32]) -> u8 { seed[0] }
EOF
cat > "$TMP/bebop2/core/src/pq_kem.rs" <<'EOF'
// ungated on purpose — simulates C3 regression
pub fn keygen_internal(d: &[u8; 32], z: &[u8; 32]) -> u8 { d[0] }
EOF
if GUARD_ROOT="$TMP" bash "$GUARD"; then
  echo "RED FAIL: guard did not catch ungated keygen"; rm -rf "$TMP"; exit 1
fi
echo "RED PASS (guard caught the regression)"
rm -rf "$TMP"
echo "ALL C3 REGRESSION CHECKS PASS"
