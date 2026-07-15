#!/usr/bin/env bash
# Build the GitHub webhook as an OPS/Nanos unikernel microVM image.
#
# Zero-OCI: no Docker, no JS — a single Rust ELF booted as a unikernel on a
# hypervisor (KVM/Firecracker via `-t hvt`), matching the sovereign-node tier
# (scripts/build-unikernel.sh, config.sovereign.json).
#
# Prereqs (operator-installed; not bundled):
#   - Rust toolchain (build the ELF)
#   - ops        : curl https://ops.city/get.sh -sSfL | sh
#   - a hypervisor with KVM/Firecracker on the boot host
#
# The webhook secret is NEVER baked into the image — pass it at instance-create
# time (see the printed command / DEPLOY.md).
set -euo pipefail
cd "$(dirname "$0")"

IMAGE="${IMAGE:-bebop-github-webhook}"

echo "── 1. build the release ELF ───────────────────────────────────────────"
cargo build --release --bin bebop-github-webhook
strip target/release/bebop-github-webhook 2>/dev/null || true
ls -la target/release/bebop-github-webhook

echo "── 2. package as a Nanos unikernel image ──────────────────────────────"
if ! command -v ops >/dev/null 2>&1; then
  echo "✗ 'ops' not installed. Run: curl https://ops.city/get.sh -sSfL | sh" >&2
  echo "  (the ELF above is built and ready; re-run this script once ops is present)" >&2
  exit 1
fi
ops image create -c ops.json -i "$IMAGE" -t hvt

cat <<EOF

✓ wrote unikernel image '$IMAGE' (.hvt).

Boot it as a microVM (inject the secret at run time — never in the image):

  ops instance create -i "$IMAGE" -t hvt \\
    -e GITHUB_WEBHOOK_SECRET="\$GITHUB_WEBHOOK_SECRET" \\
    -p 8080

Then front it with Cloudflare Tunnel on the host (cloudflared is a Go binary,
not Docker/JS) — see DEPLOY.md.
EOF
