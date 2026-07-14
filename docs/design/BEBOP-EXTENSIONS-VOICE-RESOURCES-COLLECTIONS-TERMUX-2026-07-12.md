# Bebop — User-defined extensions, native voice, resource telemetry, library collections, Termux tools

Date: 2026-07-12 · Operator: SyniakSviatoslav · Status: RESEARCH + PLAN (not implemented)
Push-plans-first: committed + pushed BEFORE any code. Companion file:
`docs/design/LIBRARIES-FOR-STARS.md` (the GitHub star-list the operator asked to
receive in Telegram).

---

## 0. Operator's expanded asks (verbatim intent, consolidated)

A. User can CREATE their OWN rules / hooks / loops / gates / prompts / settings —
   dynamic OR static. Deep customization, fully open & modifiable.
B. Full native VOICE control of agent + CLI — NO AI in the voice path (offline
   TTS + STT only).
C. Show system / OS / device RESOURCE consumption telemetry.
D. Easy file UPLOAD + BROWSE.
E. Create COLLECTIONS of reusable GitHub libraries: show name, gist, what it
   does, version, memory footprint, downloads, languages.
F. Easily SHARE / DISTRIBUTE / INSTALL collections; rename them; snapshot/backup
   collections; store them; give each a 32-bit PIXEL icon.
G. Termux-tools list to integrate locally + reverse-engineer + use for the goals
   of this prompt: Cariddi, ip-tracer, chafa, wormgpt, neovim, traceroute,
   blackbird-osint, webinfo, onefetch, aliens-eye, dufs, lynx, sentinel, octopus,
   nmap, netcat, masscan, web-tech scanning, rustscan, naabu, dns-scanning,
   discovery-workflow, spiderfoot, termux-localhost.
H. Operator's OWN tool/skill/library collection = DEFAULT, auto-enabled, but
   changeable / disableable in settings.
I. Ability to enable / disable / extend / DELETE libs from collections.
J. GitHub AUTHOR attribution + CLI reminder to STAR them.
K. Settings: open-source THANK-YOU + full author list + resources borrowed from
   (Hermes, OpenCode, Claude).
L. Collections configurable per ENVIRONMENT; telemetry on which were long-unused;
   AUTO vuln-scan BEFORE integrating a library; PERIODIC vuln/update scan.
M. Everything enable/disable/extend/delete in settings — NO blockers; user decides.

SAFETY NOTE (operator rule M + dual-use tools): several Termux tools (nmap,
masscan, rustscan, naabu, spiderfoot, Cariddi, blackbird-osint, ip-tracer) are
recon/OSINT. This is a COLLECTION/PACKAGE MANAGER for the operator's OWN dev env
+ architecture — NOT an attack tool. Therefore: (1) pre-integration vuln scan is
mandatory (cargo-deny + the property-gate CI already built); (2) NO tool runs a
network scan automatically — every recon tool is manual-enable and explicit-run
only; (3) `wormgpt` is flagged dual-use, NOT in the default collection, manual
opt-in only.

---

## 1. Research findings (evidence-backed)

### 1.1 Native voice (no AI in the path)
- TTS: `espeak-ng` (C, multilingual, tiny) or `piper` (ONNX neural, higher quality,
  still 100% local). Both CLI-first, pipe text → audio. No network.
- STT: `whisper.cpp` (ggml-org) — offline speech-to-text, mic → text. Optional
  VAD. No network, no cloud LLM.
- Integration: bebop CLI gains a `voice` subsystem that shells out to these local
  binaries (like it already shells out to cargo). The transcribed text is fed to
  the SAME command parser as typed input — voice is just another input device.
- Constraint honored: "no AI" in the voice path = we never send audio to an LLM
  for transcription; whisper.cpp runs locally. The agent reasoning still uses the
  configured model, but transcription itself is AI-free/local.

### 1.2 Resource telemetry (system/OS/device)
- Rust: `sysinfo` crate (cross-platform CPU/RAM/disk/OS/process). Already a common
  dep; add it. For device: `sysinfo` + `std::env` (OS version, arch, hostname).
- No new vendor lock; `sysinfo` is pure-Rust, MIT.

### 1.3 Library collections / sharing
- A "collection" = a TOML/JSON manifest listing GitHub repos (owner/name, version
  pin, local install path, icon). bebop resolves each to its GitHub API (stars,
  downloads, languages, license) — cached, offline-fallback to manifest metadata.
- Share = export/import the manifest file. Install = `git clone`/`cargo add` per
  entry. Snapshot/backup = copy the manifest + pinned versions + a metadata blob.
- 32-bit pixel icon = a tiny PNG (e.g. 32×32) stored in the collection dir; bebop
  renders it via `chafa` (already in the Termux list!) or a ratatui block-grid.

### 1.4 Termux tools (real repos, dual-use flagged)
See `LIBRARIES-FOR-STARS.md` for the full link list. Categories:
- Editors/browse: neovim, lynx.
- Media/render: chafa (ASCII/image), onefetch (repo summary), aliens-eye.
- File/server: dufs (file server), termux-localhost.
- Recon/OSINT (MANUAL-ENABLE, vuln-scanned): nmap, masscan, rustscan, naabu,
  dnsx, spiderfoot, Cariddi, blackbird-osint, ip-tracer, traceroute, netcat,
  webinfo, sentinel, octopus, discovery-workflow, web-tech-scanning.
- Flagged: wormgpt (dual-use; NOT in default collection; opt-in only).

---

## 2. Design — User-defined extensions (A)

New module `crates/bebop/src/extensions.rs` (debt-aware; reuses `customize` TOML
pattern + the existing `law-hooks.mjs` hook concept, ported to native Rust so it
runs without Node in the binary):

```
~/.bebop/extensions/
  rules.toml      # user logic laws (dynamic + static)
  hooks.toml      # pre/post command hooks (shell or wasm)
  loops.toml      # named loop definitions (stabilizer/opt patterns)
  gates.toml      # property gates (RED+GREEN checks)
  prompts.toml    # named prompt templates (variables, inherits)
```

- Each entry: `name`, `kind` (static|dynamic), `body`, `enabled`, `env` (which
  environments it applies to), `coefficient` (for tunable rules — matches the
  17-point plan's "rules with coefficient ranges").
- Static = literal (a fixed hook script). Dynamic = a small expression evaluated
  against live telemetry (reuses `tui::Telemetry` + `enrich::Trace` aggregates),
  e.g. `drift > 60 => warn`.
- Loaded at boot; `bebop extensions validate` checks syntax + fails closed (a bad
  user rule cannot crash the agent — it is skipped + logged).
- Hooks fire at the SAME boundaries the existing `law-hooks.mjs` already hooks
  (pre-commit, pre-dispatch, post-action).

Maps to: `customize::Profile` (TOML load/save), `law-hooks.mjs` (hook model).
No Node at runtime — port the hook runner to Rust so the binary stays Node-free
(operator rule: "TS eliminated from runtime").

---

## 3. Design — Native voice (B)

New subsystem `crates/bebop/src/voice.rs`:
- `voice listen` → shells `whisper.cpp` on the mic → transcribed text → fed to the
  command parser (same path as typed input).
- `voice speak "<text>"` → shells `espeak-ng`/`piper` → audio out.
- The agent can auto-narrate debriefs/CoT via `voice speak` when `voice.auto = true`.
- Config: `voice.tts = "espeak-ng"|"piper"`, `voice.stt = "whisper.cpp"`,
  `voice.auto = false`, `voice.model_path = "..."` (local model file).
- NO network, NO cloud LLM in transcription. Voice is an input/output device only.
- Graceful fallback: if the binary is absent, print "voice disabled: install
  espeak-ng/whisper.cpp" and continue (never a blocker).

---

## 4. Design — Resource telemetry (C)

Extend `tui::Telemetry` + a new `sysinfo` source:
- `resource` panel: CPU%, RAM used/total, disk used/total, OS + version, arch,
  hostname, uptime, bebop process RSS.
- Shown in the helm + in the debrief (per-match resource cost).
- Config: `telemetry.resources = true`.
- Pure-Rust (`sysinfo`, MIT). No new vendor.

---

## 5. Design — File upload + browse (D)

- `bebop fs browse [path]` → ratatui file-tree (reuses `tui` palette).
- `bebop fs get <remote> <local>` / `bebop fs put <local> <remote>` → uses the
  existing `dufs` (file server) or `zenoh`/local FS. Upload = local copy into the
  agent workspace; no cloud.
- Browse is read-only by default; write gated by `mode` (plan = no writes).

---

## 6. Design — Library collections (E, F, H, I, J, K, L)

New module `crates/bebop/src/collections.rs` + manifest format:

```
~/.bebop/collections/
  default.toml        # operator's default collection (auto-enabled)
  <name>.toml         # user/shared collections
  icons/<name>.png    # 32-bit pixel icon per collection
```

Collection manifest entry:
```toml
[[lib]]
owner = "RustCrypto"
name  = "ml-dsa"
version = "0.1"
what = "FIPS 204 ML-DSA signatures (PQ)"
memory_kb = 0        # dep-free core
downloads = 0        # resolved from GitHub API, cached
languages = ["Rust"]
icon = "icons/ml-dsa.png"
enabled = true
```

Operations (all in settings, no blockers):
- `bebop coll list` — name, gist, version, memory, downloads, languages, enabled.
- `bebop coll add <owner/name>[@ver]` — fetches GitHub metadata, pre-integration
  vuln scan (cargo-deny/advisory), adds to a collection. BLOCKED if vuln scan fails
  (operator rule L) — but user can override with `bebop coll add --force` (no
  blocker; user decides).
- `bebop coll rm <name>` — remove from collection (does NOT delete the lib from
  disk unless `bebop coll purge`).
- `bebop coll rename <a> <b>` — rename collection.
- `bebop coll snapshot <name>` / `bebop coll backup <name>` — copy manifest +
  pinned versions + metadata to `~/.bebop/collections/backups/`.
- `bebop coll share <name>` — print/export the manifest (shareable file).
- `bebop coll install <name>` — `git clone`/`cargo add` each enabled entry.
- `bebop coll icon <name> <png>` — set 32-bit pixel icon.
- `bebop coll star-reminder` — prints "please star:" + all authors (J).
- GitHub AUTHOR attribution shown in `bebop coll info <name>` + settings thank-you
  page (K).

Default collection (H) = operator's curated set (the Rust crates in Cargo.toml +
the borrowed-idea sources). Auto-enabled; `bebop coll disable default` opts out.

Per-environment config (L): each collection/lib can tag `envs = ["dev","termux"]`;
only matching envs load it. Telemetry: `bebop coll telemetry` shows last-used
timestamp per lib → flags long-unused. Periodic scan: `bebop coll audit` runs
cargo-deny/advisory on every enabled lib (reuses the property-gate CI).

---

## 7. Design — Termux tools integration (G)

New module `crates/bebop/src/termux.rs` — a LOCAL tool registry:
- Each tool = a manifest entry: name, repo, binary, install method (pkg/apt/git),
  category, dual_use flag, enabled (default false for recon tools).
- `bebop tool list` — shows all, with dual-use clearly marked.
- `bebop tool enable <name>` — explicit opt-in (NEVER auto-enabled for recon).
- `bebop tool run <name> [args]` — shells out locally; for recon tools, requires
  `mode != plan` AND explicit user args (no autonomous scanning).
- Reverse-engineering: where a tool's logic is reusable natively (e.g. chafa image
  → ratatui block-grid; onefetch repo-summary → bebop `outfit`/collection info),
  port the relevant pure function into bebop (debt-aware, no new vendor).
- `wormgpt` = `dual_use = true`, `in_default = false`, manual opt-in only.

---

## 8. Design — Settings & open-source thank-you (K, M)

Extend `customize::Profile` with a top-level `[extensions]`, `[voice]`,
`[telemetry.resources]`, `[collections]`, `[termux]` sections. Settings page
(`bebop settings`) shows:
- Every toggle (enable/disable/extend/delete) — NO blockers.
- An open-source THANK-YOU block: full author list of every borrowed library +
  the resources ideas were taken from (Hermes Agent, OpenCode, Claude Code,
  RustCrypto). Rendered in `mission.rs`-style banner + persisted in
  `docs/ATTRIBUTIONS.md` (generated, committed).

---

## 9. Map to existing modules (no new crate where avoidable)

| Need                    | Extend (don't rewrite)                  |
|-------------------------|-----------------------------------------|
| User rules/hooks/loops  | `customize::Profile` + port `law-hooks` |
| Voice                   | NEW `voice.rs` (shells espeak/whisper)  |
| Resource telemetry      | `tui::Telemetry` + `sysinfo` dep         |
| File upload/browse      | `tui` + `dufs`/`zenoh` (existing)       |
| Collections             | NEW `collections.rs` + `customize` TOML  |
| Termux tools            | NEW `termux.rs` (local registry)        |
| GitHub metadata         | `collections.rs` (cached API)           |
| Vuln scan gate         | reuse `scripts/ci-supply-chain.sh` logic|
| Attribution/thank-you   | `customize` + `docs/ATTRIBUTIONS.md`    |

New Rust deps: `sysinfo` (resources). Voice/tools = shell-outs (no Rust dep).
Everything else reuses what exists.

---

## 10. Implementation phases (RED→GREEN gates)

**Phase A — User extensions framework**
- A1. `extensions.rs` + `~/.bebop/extensions/{rules,hooks,loops,gates,prompts}.toml`.
- A2. Port hook-runner to Rust (replace Node `law-hooks.mjs` dependency at runtime).
- A3. Fail-closed load (bad rule skipped + logged).
- GATE: RED — malformed rule must not crash; GREEN — rule fires on its boundary.

**Phase B — Resource telemetry**
- B1. Add `sysinfo` dep; `resource` panel + debrief line.
- GATE: RED — missing sysinfo errors gracefully; GREEN — shows real CPU/RAM/OS.

**Phase C — Native voice**
- C1. `voice.rs`: `listen` (whisper.cpp) + `speak` (espeak-ng/piper).
- C2. Wire transcribed text into the command parser; auto-narrate debrief.
- GATE: RED — absent binary → graceful disable; GREEN — mic→text→command round-trip.

**Phase D — File upload/browse**
- D1. `fs browse` (ratatui tree) + `fs get/put` (local/dufs).
- GATE: RED — browse of missing path errors; GREEN — upload puts file in workspace.

**Phase E — Collections**
- E1. Manifest format + `coll list/add/rm/rename/snapshot/backup/share/install/icon`.
- E2. GitHub metadata cache + pre-integration vuln scan (force override).
- E3. Default collection auto-enabled; per-env tags; unused telemetry; periodic audit.
- GATE: RED — vuln scan blocks bad lib (force overrides); GREEN — install clones + stars shown.

**Phase F — Termux tools**
- F1. Tool registry manifest + `tool list/enable/run`; recon = manual-enable.
- F2. Reverse-engineer reusable pure logic (chafa→block-grid, onefetch→info).
- GATE: RED — recon tool never auto-runs; GREEN — explicit `tool run` works locally.

**Phase G — Settings + attribution**
- G1. Extend `Profile` with all sections; `bebop settings` UI.
- G2. Generate `docs/ATTRIBUTIONS.md` (authors + borrowed resources).
- GATE: doc-claim verifier GREEN (test counts updated).

---

## 11. Open questions (auto mode decides, records in self-review)
- Q1: Default TTS engine — espeak-ng (fast/tiny) or piper (quality)? (default: espeak-ng)
- Q2: whisper.cpp model size default? (default: base.en, local)
- Q3: Collection GitHub metadata — live API or cached-only offline? (default: cached + refresh on `coll audit`)
- Q4: Periodic vuln scan cadence? (default: on `coll add` + weekly `coll audit`)

All surfaced for transparency; in `auto` bebop picks defaults and logs them.
