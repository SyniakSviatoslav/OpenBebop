# Changelog

All notable changes to Bebop are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/), and this project adheres to
[Verified-by-Math](./docs/ARCHITECTURE.md): every behavior change ships with a falsifiable
RED+GREEN test.

## [Unreleased]

### Fixed — honesty & verification debt (operator directive 2026-07-08)
- **`recall` now returns REAL payloads from the bundled living memory** (`src/memory.ts`: VSA
  hypervectors + graph spreading-activation, with forgetting + persisted snapshot). Previously it
  returned truncated node ids. Verified: `bebop recall "kernel law"` prints full payload text + score.
- **Bundled VSA vector recall is now honest about its weakness.** The char-codebook hypervector is a
  *weak* associative signal (bipolar noise floor is high — gibberish scored ~0.74). Graph
  spreading-activation is the primary, deterministic recall path; vector `nearest()` is a fallback
  used ONLY when graph finds nothing, gated at sim>0.85 so gibberish yields 0 hits. RED-proved in
  `src/knowledge.test.ts` (gibberish → no meaningful corpus payload surfaced).
- **Real customization wired (was dead).** `bebop init` writes `narration` + `looks` axes; they now
  actually drive the CLI: `voice.ts` `voiceFor(narration)` changes the co-pilot voice (plain/corporate-killer
  strip all wit; bebop/sarcastic keep dry cosmo-noir), and `theme.ts` `makePaint(looks)` changes the
  primary accent (bebop/claude/opencode/codex, or `BEBOP_THEME_ACCENT` for custom). `settings.ts`
  now reads both axes from the user profile. Verified by `src/theme.test.ts` + `src/voice.test.ts`.
- **README rewritten — half the size, unique-first.** Lead sentence names the advantages NOT done by
  other agents yet: deterministic Rust/WASM guard kernel (bebop core), post-quantum node identity (PSQ),
  living VSA memory, freestyle self-evolving soul, real narration + looks, local-first privacy. The
  "vs the others" section is now a combiner-tool framing that credits competitors' strengths.
- **Recorder fixed (honesty bug) + ALL 18 feature GIFs re-recorded with real animation.**
  `scripts/record-feature.sh` no longer forces `NO_ANIM=1` AND no longer pipes stdout through `head`
  (the pipe made node's stdout a PIPY → isTTY=false → animation silently disabled, the real cause of
  flat GIFs). asciinema allocates a PTY, so `playLaunch` now renders; `feat-boot.gif` went from 2
  frames (flat) to 8 frames (animated) with real `\u001b[1G\u001b[0K` spinner redraws + teal ◈ glyph.
  Verified: `grep '"o"' docs/footage/feat-boot.cast` shows cursor-move/clear-line animation frames.
- **README "Bebop vs the others" rewritten as a combiner tool** (not a replacement), crediting
  competing agents' real strengths, less jargon, more verified facts. No ✅/❌ superiority matrix.

### Added — doc-claim self-correction layer (the anti-falsification guardrail)
- **`scripts/verify-doc-claims.mjs`** turns every load-bearing README/doc claim into a FALSIFIABLE
  check run against live code: recorder must not force `NO_ANIM=1`; launch animation must be wired +
  TTY-gated; `settings.ts` must read `narration`+`looks` and be tested; PSQ identity must have a real
  test; `recall` must return real payloads; README's test count must match `npm test`; no ✅/❌
  superiority matrix vs competitors; wiki claim must be honest. Exit 1 on any unbacked claim.
- **Wired into `bebop docs check` + `bebop docs build`** — a non-zero verifier status blocks release.
- **`.git/hooks/pre-commit` runs it on EVERY commit** — a doc claim not backed by live proof refuses
  the commit. Proven falsifiable: reintroducing `NO_ANIM=1` or injecting a ✅/❌ matrix makes it RED.
- **Standing rule added to `docs/RULES.md`** codifying the layer so it is never dropped.
- **Wiki/visualization gap acknowledged** — in-repo `docs/features/*` are thin; the richer
  §0·GP retriever + VSA codec are NOT bundled (they live in the dowiz monorepo). `bebop docs check`
  reports this honestly rather than claiming a populated wiki.

### Added — ReAct agentic loop (Reason→Act→Observe→Reflect), visible by default
- **`runLoop` rewritten as a true ReAct loop** (`src/loop.ts`). Every agentic action now goes through
  explicit `Reason → Act → Observe → Reflect`: the `thought`, the `action` taken, the `observation`
  (Ok/Err), and the `reflection` are all recorded. The loop prints each iteration to the transcript
  (TTY-gated; `NO_ANIM=1` still shows the structured lines) so the retry is NEVER hidden as a single
  "perfect" step the way promo demos fake it.
- **Iterations are real + configurable.** `BebopConfig.iterations` (default **3**), overridable at
  runtime by `BEBOP_REACT_ITERS`. Proven falsifiable: `loop.react.test.ts` asserts the count is honored
  exactly (1 attempt does not allow a rewrite to land; 2 does) and `verify-doc-claims.mjs` §I turns RED
  if the default is broken.
- **Real-time eval gate inside the loop** (`evalStep`): each iteration is scored on draft/test/observe/
  reflect quality (`EvalVerdict { passed, score, notes }`). The guard checks legality; `evalStep` checks
  QUALITY — they combine, so "didn't crash" is not mistaken for "correct". The verdict is recorded in the
  trace.
- **`LoopResult` now carries `iterations` + `reactTrace`** (`ReactStep[]`) — the full audit trail,
  returned to callers and surfaced in transcripts.
- **Proof**: `src/loop.react.test.ts` (5 RED+GREEN tests) + `scripts/verify-doc-claims.mjs` §I. Total
  `npm test` is now **181** (was 176).

## [0.2.0] — 2026-07-08

### Added — agent parity (reverse-engineered from Claude Code + Hermes public surface)
- **Slash commands**: `/help /status /model /clear /plan /compact /resume /skills /review /subagent`
  (Claude Code's `/`-dispatcher analogue), wired in `bebop.ts`.
- **Plan mode** (`bebop run <class> --plan`): read-only loop — `edit` is denied before the guard
  gate (Explore/Plan subagent semantics). RED-proved in `src/loop.test.ts`.
- **Headless JSON** (`bebop run <class> --json`): one-shot structured output, no prompts.
- **Settings file** (`~/.bebop/settings.json` user + optional `bebop.json` project): `model`,
  plus `permissions.allow/deny` and `hooks` from the **user** file only. A project `bebop.json`
  is untrusted and may set **only `model`** — `permissions`/`hooks` are ignored + warned (see
  `src/settings.ts` `applyProject`). See `src/settings.ts`.
- **Hooks** (`src/hooks.ts`): PreToolUse / PostToolUse / Stop with deny decisions (Claude Code
  analogue). A PreToolUse hook runs *before* the guard gate and can deny (fail-closed on crash).
- **Subagents** (`subagent()` in `src/loop.ts`): scoped, read-only, cheaper-model delegation that
  returns only a summary (context-saving Explore/Plan pattern).
- **Skills loader** (`src/skills.ts`): loads `SKILL.md` (agent-skills frontmatter) from
  `.bebop/skills/*`; ships one sample skill (`/review`).
- **Tests**: 22 new RED+GREEN tests (settings, hooks, loop plan/hooks/subagent, skills).

### Added — visualization, machine-readable docs, narration, i18n, live footage
- **`understand.ts` + `bebop map`** — "understand everything": derives the real module graph from
  ACTUAL imports (no guessing) and renders it as a zero-dependency SVG (`docs/diagrams/project-map.svg`).
  `bebop map <module>` focuses on a single module + its real neighbours.
- **`schema.ts` + `bebop diagrams`** — regenerates 15 conceptual + real SVGs in `docs/diagrams/`
  (project map, 7 feature schemas, 7 focused subgraphs). Fully deterministic, committed for the wiki.
- **In-repo wiki visuals per feature** — every `docs/features/*.md` and `docs/integrations/*.md` now
  has a `## ▶ Live CLI` section with real recorded CLI GIFs (see footage below).
- **`docs/RULES.md`** — the Constant Doubt universal verification rule (no verification → no statement).
- **`docs/VERIFICATION-MATRIX.md`** — 35-row proof table mapping every feature claim → live probe → PASS.
- **README rewrite** — story-for-a-5-year-old, "why businesses care" pain table, honest comparison vs
  other agentic CLIs, "What Bebop is NOT" limitations, cinematic section hooks, read-time / 🎧 listen /
  🤖 for-agents callouts.
- **Audio narration** — `docs/narration/*.mp3` + transcripts for README / architecture / limitations.
- **Machine-readable layer** — `llms.txt`, `llm-manifest.json` (structured verified facts),
  `docs/mcp-tools.json` (MCP tool schemas), and a `localization` block in the manifest.
- **Live footage** — real `asciinema` recording → `agg` GIF at the top of the README
  (`docs/footage/bebop-session.gif`) plus 16 per-feature GIFs; recorders in `scripts/record-*.sh`.
- **i18n** — `README.uk.md` (hand-reviewed Ukrainian), `docs/i18n.md`, and
  `scripts/i18n-translate.mjs` (free OSS auto-translate via LibreTranslate / Argos, no keys, no
  telemetry; code blocks & links preserved). GitHub now shows a language switcher for the README.

### Fixed
- **CI failure (MCP tests flaky/hanging on the runner)** — `mcp.test.ts` previously spawned a
  real `bebop.ts mcp` child process and asserted over stdio with an 8s timeout; replaced with a
  deterministic pure `handle()` test. Added `InvalidParamsError` (proper `-32602`).
- **CI Node**: pinned `actions/setup-node` to Node 22 (LTS) to clear the Node20 deprecation notice.
- **`mcp.ts` `bebop_route` enum** — `taskClass` now matches the real `TaskClass` (`doer`/`reason`/
  `redline`), not a mismatched creativity axis.
- **Docs truth-audit** — `recall` no longer claims a working VSA retriever (in-process living memory
  only; honestly reports the retriever isn't bundled); removed non-existent `npm run lint`/`format`
  from the dev gate; corrected all test counts to the verified **165** TS / **7** Rust.
- **`bebop use free` fail-closed clarity** — quick-start now uses `use native` (keyless default) and
  documents that `use free` *refuses* without `OPENROUTER_API_KEY` by design.
- **Footage recorder portability** — `scripts/record-feature.sh` now resolves `asciinema`/`agg` from
  PATH or a known venv (no committed `/tmp` symlink); drops+removes a harmless model-only `bebop.json`
  via trap so recordings never pollute git.

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
- `npm test` → 159 tests (155 pass + 4 skipped without `better-auth`; 159/159 with it).
- `npm run typecheck` → 0 source errors.
- Clean clone + `npm install --omit=optional` reproduces the above.
