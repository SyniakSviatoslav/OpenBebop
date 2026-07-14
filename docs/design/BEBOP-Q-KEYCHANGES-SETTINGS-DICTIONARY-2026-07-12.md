# Q — Key-changes visibility · destructive/critical display · settings dictionary

Date: 2026-07-12 · Operator: SyniakSviatoslav · Status: DESIGN (push-plans-first)

## Background (operator asks, verbatim)
- "і показ ключових змін чи дій як у hermes хороша фіча - запозич, інтегруй і зроби налаштовуваною"
- "і показ деструктивних чи критичних змін також у cli - також із налаштуванням того що вважається деструктивною чи критичною зміною"
- "А ще усі налаштування мають мати зрозумілий словник, щоб агенти всередині cli сам міг крутити налаштування за запитом користувача"

## 1. What Hermes does (reverse)
Hermes CLI prints a compact per-action line before/after a tool call, e.g.:
`◆ edited  crates/bebop/src/foo.rs`  /  `✎ wrote  path`  /  `↳ ran  command`.
It shows the ACTION VERB + TARGET + (optionally) a one-line diff summary. Non-verbose,
scannable, color-coded. That is the "key changes/actions" surface.

## 2. Design (native Rust, offline, deterministic)

### 2.1 `changes.rs` — Hermes-style change record (KEY CHANGES)
```
pub enum ChangeKind { Create, Edit, Delete, Run, Config, Git }
pub struct ChangeRecord { kind: ChangeKind, target: String, summary: String, destructive: bool }
pub fn render_changes(records: &[ChangeRecord]) -> String  // compact, line-per-change, verb + target
```
- The agent loop appends a `ChangeRecord` for every mutating action it takes (file write,
  command run, config set, git push). `render_changes` emits a Hermes-like scannable log.
- Tests: RED (empty → empty render) + GREEN (edit+create render with verb+target).

### 2.2 `destructive.rs` — configurable destructive/critical classifier (CRITICAL DISPLAY)
```
pub struct DestructivePolicy { patterns: Vec<String>, labels: Vec<String> }  // user-tunable
pub fn classify(policy: &DestructivePolicy, rec: &ChangeRecord) -> Severity  // None | Destructive | Critical
```
- Default policy flags: `git push --force`, `git reset --hard`, `rm -rf`, `drop table`,
  `delete`, file-overwrite outside sandbox, red-line (auth/money/RLS) as CRITICAL.
- `render_changes` promotes Destructive/Critical records to a BLOCK (⚠ + label) so the user
  sees them prominently before confirm. Configurable: user edits `policy.patterns`.
- Tests: RED (benign edit → None) + GREEN (force-push → Critical; rm -rf → Destructive).

### 2.3 `settings.rs` — SETTINGS DICTIONARY (self-service)
```
pub struct SettingEntry { key, description, default, allowed: Vec<String>, current }
pub fn dictionary() -> Vec<SettingEntry>        // ALL settings, with clear human description
pub fn get(key: &str) -> Option<String>
pub fn set(key: &str, val: &str) -> Result<(), String>   // validate against allowed
```
- Every axis from this whole effort is an entry: gender, profanity, archetype, god_relation,
  lanes_on, max_lanes, auto_intent, destructive_policy, change_visibility, ...
- `dictionary()` returns a human-readable list the CLI prints on `bebop settings list`,
  and an AGENT can call `set()` per a user request ("switch profanity to forbidden").
- Tests: RED (unknown key → Err) + GREEN (set gender=neuter validates; get returns).

## 3. Integration
- `cli.rs`: `bebop settings list` / `bebop settings get <k>` / `bebop settings set <k> <v>`.
- `agent_loop.rs`: after each step, append `ChangeRecord`, render at end (or streaming).
- `customize.rs` Profile: persist `destructive_policy` + `change_visibility` overrides.

## 4. What NOT to do (YAGNI)
- No LLM in the classifier. Pattern/regex only.
- No external telemetry. Local log only.
- No new deps.

## 5. Verification
- `cargo test -p bebop changes::` + `destructive::` + `settings::` RED+GREEN.
- doc-claim verifier stays GREEN.
