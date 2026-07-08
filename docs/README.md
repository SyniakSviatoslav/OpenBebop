# Bebop — Wiki

Welcome to the Bebop wiki. Bebop is a self-hostable coding agent with a deterministic guard
operating system, living memory, post-quantum node identity, and a math-proven telemetry
governor. This wiki explains each subsystem in detail so you can fork, extend, and trust it.

## Start here
- [Getting started](./getting-started.md) — install, run, configure.
- [Architecture](./ARCHITECTURE.md) — the layers, the Rust/WASM guard kernel, the determinism contract, "as above so below".
- [Command reference](./commands.md) — every `bebop` subcommand.

## Key features (deep dives)
- [Guard OS](./features/guard-os.md) — the deterministic gate that refuses to lie.
- [Deterministic kernel & content-addressed log](./features/kernel.md) — decide/fold/replay + the Checker gate + hash-chained store.
- [Telemetry governor](./features/governor.md) — PID + ICIR + resonance; autonomy as a control loop.
- [Living memory (VSA)](./features/memory.md) — Vector Symbolic Architecture: insert, forget, recall.
- [Post-quantum identity & vault](./features/identity.md) — ML-KEM + Ed25519, self-certifying, encrypted at rest.
- [No-central-server mesh](./features/mesh.md) — content-addressed, verified pieces; swap-not-rewrite.
- [Freestyle bebop soul](./features/consciousness.md) — self-maintenance, self-evolution, session-as-node.

## Integrations
- [MCP server](./integrations/mcp.md) — plug Bebop into Claude Desktop, Cursor, Zed, VS Code, Hermes.
- [Backends & routing](./integrations/backends.md) — bring your own model; cheapest-adequate routing.
- [Sync (optional)](./integrations/sync.md) — self-hosted Better Auth node for multi-device.

## Project
- [Contributing](../CONTRIBUTING.md) · [Governance](../GOVERNANCE.md) · [DCO](../DCO.md) · [Security](../SECURITY.md)
- [License](../LICENSE) — AGPL-3.0-or-later.
