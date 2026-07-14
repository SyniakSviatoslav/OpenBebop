#!/usr/bin/env bash
# G3 guard (2026-07-14): the mesh sync topic must be a canonical proto-cap
# variant (Resource::Sync / Action::Pull), NOT a placeholder carrier. The
# original code mapped SyncScope::to_capability_scope to Ledger::Read as a
# temporary stand-in, splitting the topic taxonomy (sync_pull.rs::SyncResource
# vs scope.rs::Resource). This guard fails if the placeholder mapping survives.
set -euo pipefail

REPO_ROOT="${GUARD_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}"
SRC="$REPO_ROOT/bebop2/proto-wire/src/sync_pull.rs"

if ! grep -q 'Scope::single(Resource::Sync, Action::Pull)' "$SRC"; then
  echo "G3 VIOLATION: SyncScope::to_capability_scope does not map to the canonical (Resource::Sync, Action::Pull) topic."
  exit 1
fi
if grep -q 'Scope::single(Resource::Ledger, Action::Read)' "$SRC"; then
  echo "G3 VIOLATION: placeholder sync-topic carrier (Ledger::Read) still present in sync_pull.rs."
  exit 1
fi
echo "G3 guard: OK (sync topic unified under canonical Resource::Sync / Action::Pull)."
