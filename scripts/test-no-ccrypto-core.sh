#!/usr/bin/env bash
# RED+GREEN regression for the B4 C-crypto fence.
set -euo pipefail
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
GUARD="$REPO_ROOT/scripts/ci-core-no-ccrypto.sh"

echo "== GREEN: real repo core is C-crypto-free =="
bash "$GUARD" && echo "GREEN PASS" || { echo "GREEN FAIL"; exit 1; }

echo "== RED: injecting a C-crypto dep into bebop2-core must trip the guard =="
CORE_TOML="$REPO_ROOT/bebop2/core/Cargo.toml"
cp "$CORE_TOML" "$CORE_TOML.b4bak"
# Insert a ring dependency into the core manifest (mutation under test).
if grep -q '^\[dependencies\]' "$CORE_TOML"; then
  sed -i '/^\[dependencies\]/a ring = "0.17"' "$CORE_TOML"
else
  printf '\n[dependencies]\nring = "0.17"\n' >> "$CORE_TOML"
fi
# cargo tree needs a resolved dep; try, but even without full resolve the guard's
# static scan of the manifest-backed tree will surface `ring`. If cargo tree fails
# to resolve (no lock entry), we still assert the guard flags the injected dep.
if bash "$GUARD"; then
  echo "RED FAIL: guard did not catch ring added to bebop2-core"
  mv "$CORE_TOML.b4bak" "$CORE_TOML"
  exit 1
fi
echo "RED PASS (guard caught the regression)"
mv "$CORE_TOML.b4bak" "$CORE_TOML"
echo "ALL B4 REGRESSION CHECKS PASS"
