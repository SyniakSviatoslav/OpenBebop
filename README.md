# ◈ Bebop

> Your kitchen, your ship, your cut.
> A standalone coding-agent CLI that drives **any** connected agentic CLI — Claude Code, Codex, OpenCode, Hermes, Aider, Goose — behind one guard kernel, with **free LLMs by default** and a simple switch to any of them.

Bebop is a complete, independent tool. Its own trust boundary (a Rust/WASM guard kernel), its own retriever (VSA), its own token router, and its own copilot (doer→checker) live **in this repo** — no other project required.

- **License:** AGPL-3.0
- **Runtime:** Node 22 LTS (no `better-auth` required for the CLI)
- **Brand:** Warm Cosmo-Noir. Main signal color: ship teal `#46B0A4`.

---

## Why Bebop

Bebop is the abstraction **above** other agents. You don't marry one CLI — you point Bebop at whichever is connected and switch on a whim:

```
bebop agents          # what's connected right now
bebop use claude      # switch the default agent directly
bebop use opencode    # ...or codex, hermes, aider, goose, free, native
```

Every agent runs **behind the same guard**: a red-line deny set (auth / money / RLS / migrations / secrets), a scope allow-list, and a deterministic doer→checker copilot. The intelligence (routing, guard, tokens, memory) is Bebop's; the agent is a dumb executor.

## Free by default

Bebop runs on **free LLMs by default** — OpenRouter's free tier (e.g. `mistralai/mistral-7b-instruct:free`). All you need is a free OpenRouter key:

```
export OPENROUTER_API_KEY=sk-or-v1-...   # free tier, no credit card
bebop dispatch "refactor tools/bebop/loop.ts"
```

No key? Bebop still boots and runs — the conductor falls through to the **keyless native loop** (a deterministic stub), so you are never hard-blocked. To plug in a paid model or another CLI, just set its key; `bebop agents` shows what's live.

---

## Install

```bash
git clone https://github.com/SyniakSviatoslav/bebop
cd bebop
npm install
npm run build        # compiles the Rust/WASM guard kernel → src/bebop_core.wasm
npm link            # or: npx tsx bebop.ts <cmd>
```

> The WASM kernel artifact (`src/bebop_core.wasm`) is committed, so the CLI works even without a Rust toolchain. To rebuild it: `cd crates/core && bash build.sh`.

## Quick start

```bash
bebop boot            # guard self-test — refuses to start if the gates can't go RED
bebop status          # shows the agent rotation + what's connected
bebop agents          # every agentic CLI Bebop can drive, with live status
bebop use free        # (default) free LLMs
bebop dispatch "fix the red ship animation"   # runs behind the guard + copilot
bebop run doer        # full agentic loop (deterministic native stub by default)
```

---

## Commands

| Command | What it does |
|---|---|
| `boot` | Guard self-test. Red-line + scope gates must deny on bad and pass on good (Verified-by-Math). Refuses to start otherwise. |
| `status` | Agent rotation + connection status (free first by default). |
| `agents` | List **every** agentic CLI Bebop can drive, with live connection status + the switch command. |
| `use <backend>` | Switch the default agent directly and persist it. Refuses an unconnected backend unless `--force`. |
| `run [doer\|reason\|redline]` | Full agentic loop. Routes the task class to the cheapest adequate model lane. |
| `dispatch "<task>"` | One-shot task through the guard + copilot. Red-line tasks are **denied before any agent runs**. |
| `route <class>` | Show the token-router decision for a task class. |
| `recall <query>` | Query the living-knowledge retriever (VSA embeddings). |
| `govern "<0.9,0.6,...>"` | L5 telemetry governor (PID authority + ICIR + resonance) over a quality stream. |
| `self [maintain\|evolve\|session\|loop]` | Self-maintenance / self-evolution (fail-closed, reversible). |
| `node` | Encrypted-at-rest node identity (PQ + Ed25519). |
| `mcp` | Model Context Protocol server over stdio (zero new deps). |
| `init` | 5-axis personalization wizard → `~/.bebop/settings.json`. |
| `help` | This list. |

---

## Architecture (one paragraph)

Bebop is a TypeScript shell over a **Rust/WASM guard kernel**. The shell owns cross-cutting policy — guard (red lines + scope), token router, copilot (doer→checker), memory (VSA retriever + living knowledge), and the L5 governor. Agents (Claude/Codex/OpenCode/Hermes/Aider/Goose/`free`/`native`) are thin adapters the conductor rotates through. The kernel (`crates/core`, compiled to `bebop_core.wasm`) is a hand-rolled C-ABI module with **no wasm-bindgen**; the TS loader (`src/core-wasm.ts`) instantiates it with zero dependencies and the guard delegates to it when present, falling back to a faithful TS port otherwise.

See [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) for the full map.

## Security model

- **Project `bebop.json` is untrusted.** It may set ONLY `model`. It cannot set `permissions` or `hooks`.
- **`permissions` / `hooks` come only from `~/.bebop/settings.json`** (user-owned, trusted).
- **Hooks run without a shell** (argv split); any command containing shell metacharacters is refused.
- **Red-line globs** (auth / money / RLS / migrations / secrets / `.env`) are denied unless a human explicitly approves. The Rust kernel enforces this; probes confirm it denies `packages/db/migrations/**`, `auth/**`, `.env`, etc.

## Verification (what actually runs)

This is not a claim sheet — every statement above is exercised by the test suite (`npm test`, 159 tests) and by live probing:

- **Guard RED+GREEN:** `bebop boot` certifies the gates deny on red, pass on green. The kernel test denies `auth/token` (`kind: redline`) and allows `tools/bebop/x.ts` (`kind: ok`).
- **Dispatch denial is real:** `bebop dispatch "edit packages/db/migrations/002_users.sql"` exits non-zero with `⛔ DENIED by guard (rust)`. (A regression test spawns the real CLI and asserts this.)
- **Free default is real:** `bebop status` shows `free → … → native`; with a key, `dispatch` issues a real OpenRouter call (verified: returns the live API response, not a stub).
- **Switch is real:** `bebop use native` persists `native` as default-first; `bebop use claude` (unconnected) is refused unless `--force`.
- **Kernel parity:** when the WASM kernel is loaded, `guard.ts` agrees with the TS port on both RED and GREEN cases (parity test).

## Development

```bash
npm run lint && npm run typecheck && npm test   # gates
npm run format
cd crates/core && bash build.sh                  # rebuild the WASM kernel
cargo test -p bebop-core                         # Rust kernel unit tests (7 RED+GREEN)
```

## License

AGPL-3.0. Contributions via DCO (`git commit -s`).
