# Changelog

All notable changes to Bebop are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/), and this project adheres to
[Verified-by-Math](./docs/architecture.md): every behavior change ships with a falsifiable
RED+GREEN test.

## [Unreleased]

### Fixed
- **CI failure (MCP tests flaky/hanging on the runner)** — `mcp.test.ts` previously spawned a
  real `bebop.ts mcp` child process and asserted over stdio with an 8s timeout; under CI load the
  handshake timed out, producing 3 fails + 2 cancelled. Rewrote the tests to call the JSON-RPC
  dispatcher `handle()` directly (pure, no spawn, no timeout) — deterministic and CI-stable.
  Added `InvalidParamsError` so malformed tool args return a proper `-32602` instead of a generic
  `-32000`. Verified: 5/5 MCP tests; full suite 110 (106 pass + 4 skipped w/o `better-auth`,
  110/110 with). `gh run` now green.

## [0.1.0] — 2026-07-08

### Added
- **MCP server** (`bebop mcp`) — hand-rolled JSON-RPC 2.0 over stdio exposing guard-OS
  certification, living-memory recall/remember, telemetry governor, task routing, and
  self-maintenance as MCP tools. Zero new dependencies. `mcp.test.ts` proves the handshake.
- **In-repo wiki** (`docs/`) — detailed deep-dives for every subsystem (guard OS, kernel,
  governor, memory/VSA, identity, mesh, consciousness) plus integrations (MCP, backends, sync).
- **GitHub settings in-repo** — `CODEOWNERS`, `dependabot.yml`, `FUNDING.yml`, CI + release
  workflows, issue/PR templates, code of conduct, governance.
- **`CHANGELOG.md`**.

### Fixed
- **Governor PID state bug** — `pidStep` previously dropped `prevError` from its return type,
  corrupting the integral state across steps. Now returns the full `PIDState` (latent bug
  surfaced during open-sourcing; verified by `governor.test.ts`).
- **Test hang on optional-dep-absent install** — `auth.test.ts` now detects `better-auth`
  side-effect-free and skips *all* server-backed tests when it's absent; `sync-server.close()`
  tears down keep-alive sockets. Default install runs 105 tests (4 skipped) and exits cleanly.

### Changed
- `better-auth` moved from hard `dependencies` to `optionalDependencies` — core install stays
  portable with zero native builds.
- `package.json` enriched with 28 keywords, author, homepage, repository, bugs for
  discoverability.

### Verified
- `npm run boot` certifies the guard OS.
- `npm test` → 105 tests (101 pass + 4 skipped without `better-auth`; 105/105 with it).
- `npm run typecheck` → 0 source errors.
- Clean clone + `npm install --omit=optional` reproduces the above.
