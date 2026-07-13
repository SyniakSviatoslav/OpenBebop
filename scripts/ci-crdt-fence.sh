#!/usr/bin/env bash
# CI GUARD — CRDT-PERIPHERY COMPILE-FENCE (MESH-08).
# "we-don't-do-CRDT-for-money/orders" -> "it-doesn't-compile".
# Build FAILS if any crate that touches order/money-state depends on a
# CRDT-merge crate (automerge / cr-sqlite).
set -euo pipefail
cd "$(dirname "$0")/.."
while IFS= read -r tf; do
  dir="$(dirname "$tf")"
  # Only crates whose source mentions order/money/ledger state are in scope.
  if grep -rqE '\b(order_machine|money|ledger|claim_machine|MeshEvent|assert_transition)\b' "$dir/src" 2>/dev/null; then
    if grep -qE 'automerge|cr-sqlite' "$tf"; then
      echo "CRDT-FENCE violation: $tf touches money/order state AND depends on a CRDT crate"; exit 1
    fi
  fi
done < <(find . -name Cargo.toml -not -path '*/target/*' 2>/dev/null || true)
echo "PASS: CRDT-PERIPHERY fence — no money/order crate depends on a CRDT-merge crate."
