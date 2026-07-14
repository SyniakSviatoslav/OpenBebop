#!/usr/bin/env bash
# RED+GREEN regression for the G3 guard.
set -euo pipefail
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
GUARD="$REPO_ROOT/scripts/ci-no-placeholder-synctopic.sh"

echo "== GREEN: clean tree maps sync scope to canonical Sync::Pull =="
bash "$GUARD" >/dev/null 2>&1 || { echo "GREEN FAILED"; exit 1; }
echo "GREEN OK"

echo "== RED: a placeholder Ledger::Read carrier must trip the guard =="
TMP="$(mktemp -d)"
mkdir -p "$TMP/bebop2/proto-wire/src"
# A sync_pull.rs whose to_capability_scope still uses the placeholder carrier.
cat > "$TMP/bebop2/proto-wire/src/sync_pull.rs" <<'EOF'
fn to_capability_scope(&self) -> Scope {
    Scope::single(Resource::Ledger, Action::Read)
}
EOF
if GUARD_ROOT="$TMP" bash "$GUARD" >/dev/null 2>&1; then
  echo "RED FAILED: guard did not catch the Ledger::Read placeholder carrier"
  rm -rf "$TMP"
  exit 1
fi
echo "RED OK (guard tripped on placeholder carrier)"
rm -rf "$TMP"
echo "G3 regression: PASS (RED+GREEN)"
