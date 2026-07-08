# AGENTS.md — Bebop

Bebop is a standalone AGPL-3.0 coding-agent CLI. Operating rules for any agent (Claude Code,
Hermes, Codex, OpenCode, Aider, or Bebop itself) working in this repo.

## Hard rules (from docs/RULES.md — non-negotiable)
- **Constant Doubt**: no statement in docs is true unless backed by a live probe or a
  deterministic test. Unverified = false. Ship the RED case alongside the GREEN.
- **Verified-by-Math**: every behavior change ships with a falsifiable RED+GREEN test.
- **Red lines** (per-change human gate, never auto-touch without confirmation): auth, money,
  RLS/migrations, secrets, bulk edits.

## Repo layout
- `bebop.ts` — CLI entry (subcommands: boot, run, agents, use, recall, route, map, diagrams,
  **docs**, mcp, self, init, and the `/`-slash commands).
- `src/` — guard OS (`guard.ts`), Rust/WASM kernel (`core-wasm.ts` + `crates/core`), living
  memory, governor, routing, backends, MCP server, skills/hooks/subagents.
- `docs/` — the in-repo wiki (features, integrations, diagrams, footage, narration).
- `scripts/` — diagram + footage + i18n generators.

## Documentation pipeline (`bebop docs`)
The polished, repeatable doc-release flow. Run before any main release:
- `bebop docs build` — typecheck + tests + wasm + diagrams + map + i18n parity (no LLM needed).
- `bebop docs check` — release-readiness audit (gifs resolve, manifests valid, version semver,
  OpenWiki wired). Exits non-zero if anything is off.
- `bebop docs init` / `bebop docs update` — generate/refresh the **OpenWiki** agent-facing wiki
  in `openwiki/` (needs an LLM key: set `OPENWIKI_PROVIDER` + `OPENWIKI_API_KEY`).

## Agent-facing wiki (OpenWiki)
This repo uses [OpenWiki](https://github.com/langchain-ai/openwiki) to maintain a structured,
agent-readable wiki under `openwiki/`. **When you need durable repo context that isn't in this
file, consult `openwiki/` first** rather than re-deriving it. The wiki is regenerated on a daily
CI schedule (`openwiki-update.yml`) and is kept in sync with `git` diffs — treat it as living
documentation, not gospel; verify non-trivial claims against code.

## Verify before claiming done
- `npm run boot` — guard-OS self-certification (must go RED to be trusted).
- `npm test` — 165 falsifiable tests.
- `npm run typecheck` — clean.
- After any doc change: `bebop docs check`.
