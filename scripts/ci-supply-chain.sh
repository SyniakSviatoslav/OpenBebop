#!/usr/bin/env bash
# ci-supply-chain.sh — property-gate CI for the Rust supply chain.
#
# Per REMEDIATION-BLUEPRINT-2026-07-12.md §3G:
#   * `cargo deny fetch` then `cargo deny check` (advisories + bans + licenses + sources).
#   * `cargo audit` for vulnerability severity (independent of cargo-deny).
#   * A RED leg that proves the gate is NOT a label-gate: it runs cargo-deny
#     against scripts/fixtures/deny-bans-openssl.conf, which bans a crate the
#     committed Cargo.lock actually contains. That run MUST fail (non-zero); if
#     it ever passes, the gate is broken and CI aborts.
#
# Fail-closed: any unhandled tool failure aborts the script (set -euo pipefail).
# The script assumes `cargo-deny` and `cargo-audit` are on PATH (CI installs them
# via `cargo install -q cargo-deny cargo-audit` if missing — see the GitHub
# Actions step in .github/workflows/ci.yml).
set -euo pipefail

cd "$(dirname "$0")/.."
ROOT="$(pwd)"

echo "==> [supply-chain] ensuring cargo-deny / cargo-audit present"
for tool in cargo-deny cargo-audit; do
    if ! command -v "$tool" >/dev/null 2>&1; then
        echo "    installing $tool ..." >&2
        cargo install -q "$tool" --locked
    fi
done

echo "==> [supply-chain] cargo deny fetch (populate advisory DB; no false-green)"
cargo deny fetch

echo "==> [supply-chain] GREEN: cargo deny check (advisories/bans/licenses/sources)"
cargo deny check
echo "    cargo deny check: PASS"

echo "==> [supply-chain] cargo audit (vulnerability severity)"
cargo audit --deny warnings
echo "    cargo audit: PASS"

echo "==> [supply-chain] RED leg: prove the gate is a real property-gate"
RED_CONF="$ROOT/scripts/fixtures/deny-bans-openssl.conf"
if cargo deny check --config "$RED_CONF" bans; then
    echo "    RED LEG FAILED: denying a present crate did NOT fail — gate is broken!" >&2
    exit 1
fi
echo "    RED leg OK: denying a present crate fails the check (gate is property-complete)"

echo "==> [supply-chain] ALL CHECKS PASSED"
