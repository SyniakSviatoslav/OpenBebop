# Deploying the GitHub webhook — external adapter (`bebop2`) + Cloudflare Tunnel

**This is a `bebop2` (protocol) port, not the `bebop` agent.** OpenBebop is two
artifacts in one repo: `crates/bebop/` is the agentic coding CLI; `bebop2/` is
the from-scratch, zero-dependency protocol layer. This daemon
(`bebop2/ports/github/`) lives on the `bebop2` side as an **external-adapter
port** — std, networked, deliberately excluded from the core wasm workspace
(see the `exclude` comment in the top-level `Cargo.toml`) so it doesn't affect
`bebop2-core`'s empty-import guarantee. It only verifies GitHub's HMAC
signature and hands the event to a `WebhookSink` — it does not itself contain
Copilot Extension / agent logic. Whatever consumes the events (most likely
`crates/bebop/`) is a separate integration point.

**As actually deployed today (`dowiz-dev`, Hetzner):** a hardened **systemd
service** (`bebop-github-webhook.service`) running the built ELF directly on
the host — `DynamicUser`, sandboxed, 128MB memory cap, `127.0.0.1:8080` only,
starts on boot. Secret lives root-only at `/etc/bebop-github-webhook.env`. **No
microVM/unikernel is in production** — the Nanos/OPS path below (`ops.json`,
`build-microvm.sh`) is a real, buildable alternative for the sovereign-node
tier but has not been used to deploy this port; treat it as documented-but-
untested rather than "how it runs."

Zero-OCI either way: **no Docker, no JS.** **Cloudflare Tunnel** (the
`cloudflared` Go binary — not Docker, not JS) runs on the host and gives the
daemon a stable public HTTPS hostname with no public IP and no inbound
firewall port.

```
GitHub ──HTTPS──▶ Cloudflare edge (TLS) ──tunnel──▶ cloudflared (host, Go) ──http──▶ daemon :8080
                                                                    (today: systemd, host-native;
                                                                     documented alt: Nanos microVM)
```

## Prerequisites (operator-installed; not bundled)
- Rust toolchain (build the ELF).
- `cloudflared` (host binary) and a domain on your Cloudflare account.
- **Fresh, least-privilege** Cloudflare creds — this flow needs only a *tunnel
  token*, not an account API token and not R2 keys. Rotate anything pasted before.
- Only if using the documented-but-untested microVM alternative: `ops` —
  `curl https://ops.city/get.sh -sSfL | sh` — and a host with **KVM/Firecracker**
  (`/dev/kvm`).

## Steps (systemd — what's actually running)

**1. Pick the webhook secret:**
```bash
export GITHUB_WEBHOOK_SECRET="$(openssl rand -hex 32)"
```

**2. Build the daemon and install it as a systemd service:**
```bash
cd bebop2/ports/github
cargo build --release -p bebop-port-github
install -m 755 target/release/bebop-port-github /usr/local/bin/bebop-github-webhook
install -m 600 /dev/stdin /etc/bebop-github-webhook.env <<< "GITHUB_WEBHOOK_SECRET=$GITHUB_WEBHOOK_SECRET"
# then a systemd unit (DynamicUser, MemoryMax=128M, ExecStart binds 127.0.0.1:8080,
# EnvironmentFile=/etc/bebop-github-webhook.env) — see `systemctl cat bebop-github-webhook`
# on dowiz-dev for the exact live unit; not yet committed to this repo.
```
The secret is injected via `EnvironmentFile` — **never baked into the binary**.
The daemon fail-closes (won't serve) if the secret is empty/absent.

<details>
<summary>Alternative: microVM (OPS/Nanos) — documented, not in production</summary>

```bash
cd bebop2/ports/github
./build-microvm.sh                       # builds the ELF, packages the Nanos image
ops instance create -i bebop-github-webhook -t hvt \
  -e GITHUB_WEBHOOK_SECRET="$GITHUB_WEBHOOK_SECRET" \
  -p 8080
```
Same fail-closed behavior; injected at boot, never baked into the image.
</details>

**3. Create the Cloudflare Tunnel** (dashboard → Zero Trust → Networks → Tunnels
→ Create → *Cloudflared*), copy the tunnel token, and add a **Public Hostname**:
- Subdomain/domain: `webhook.your-domain.com`
- Service: `HTTP` → `localhost:8080` (the daemon's bound port on the host)

**4. Run cloudflared on the host** (Go binary, no Docker/JS):
```bash
cloudflared tunnel --no-autoupdate run --token "$CLOUDFLARE_TUNNEL_TOKEN"
```

**5. Configure the GitHub webhook / marketplace listing:**
- **Payload URL:** `https://webhook.your-domain.com/`
- **Content type:** `application/json`
- **Secret:** the same `GITHUB_WEBHOOK_SECRET` from step 1
- Subscribe to the events you want.

GitHub sends a `ping` immediately; the daemon prints `event=ping delivery=<uuid>
bytes=<n>` and returns `204`. A wrong/absent signature is rejected `401` and never
reaches the log — the fail-closed HMAC gate.

## The URL to list
```
https://webhook.your-domain.com/
```
Substitute your real hostname from step 3. Stable while the daemon + tunnel run.

## Marketplace plan: Free
The listing is **Free for all** — no paid tier, so the Developer Agreement's §6
(Paid Applications: merchant-of-record, 95/5 split, $500 payout minimum) doesn't
apply. Two obligations from the agreement apply regardless of price and aren't
satisfied by this port on its own:
- **§3.4 (generative-AI products)** — disclose to end users that they're
  interacting with AI-generated output, and expose a feedback path for bad/
  incorrect results. This webhook receiver is transport-only; whoever builds
  the Copilot Extension UI on top of it owns this disclosure.
- **Data Protection Addendum** — webhook payloads (`push`, `pull_request`,
  etc.) can carry personal data (commit author name/email). This port doesn't
  persist payloads itself (`WebhookSink` just hands them to the caller); the
  retention/deletion/no-resale obligations land on whichever sink stores them.

## Notes
- The daemon speaks plain HTTP only between cloudflared and the daemon on the
  host; the public hop is always HTTPS at Cloudflare's edge.
- Keep Cloudflare body transformation/compression off for this route so the raw
  bytes GitHub signed reach the daemon unchanged (default; nothing to do).
- Rotate `GITHUB_WEBHOOK_SECRET` by updating `/etc/bebop-github-webhook.env` and
  restarting the service (`systemctl restart bebop-github-webhook`), then
  updating GitHub's webhook config with the same new value. (Recreate the
  instance instead, with a new `-e` value, if using the microVM alternative.)
- Local smoke test: run the ELF directly —
  `GITHUB_WEBHOOK_SECRET=… cargo run -- 127.0.0.1:8080` — and POST to it.
