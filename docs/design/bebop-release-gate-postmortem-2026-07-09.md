# Bebop Release-Gate Postmortem + Handoff — 2026-07-09

> Self-contained. Next session can resume from here without this chat. Operator directive:
> "after an actual fix, prepare a handoff... reflect on this session, find patterns, integrate
> findings to not repeat the same mistakes."

## STATUS (verified on remote, not just local)
- **GitHub Release v0.3.5 = Latest**, published. The repo's "current release is 0.2" problem is FIXED.
- Release run `29009601037` (v0.3.5): `conclusion: success`, tests `# pass 347, # fail 0, # skipped 4`.
- `main` CI (`ci.yml`): green. No newer failing test/release run on `main`.
- Only red workflows on `main`: `openwiki-update.yml` (workflow-file/permission issue, NOT tests) — out of scope.

## ROOT CAUSE (the actual bug behind "release stuck at 0.2")
`release.yml` ran on `node-version: '20.19'`. On that Node, `node --test` does NOT expand the
quoted glob `'src/**/*.test.ts'` — glob expansion needs `globstar`, which is OFF in CI's
non-interactive `bash -e`. The Falsifiable-tests step died with:
```
Could not find '/home/runner/work/bebop/bebop/src/**/*.test.ts'
```
So **every v0.3.0–v0.3.4 release run failed at the test step** and NO GitHub Release was ever
created — even though `ci.yml` (Node 22) was green. Classic "green on one CI path, red on another."

## THE FIX (shipped in v0.3.5, commit 43d1f1d)
1. `package.json`: `"test": "node --test --import tsx $(find src -name '*.test.ts')"` —
   shell-independent (no `globstar` dependency), works on Node 20 AND 22.
2. `release.yml`: `node-version: '22'` (matches the green CI run).

## PATTERNS / MISTAKES THIS SESSION (so they don't repeat)
1. **CI vs Release Node-version mismatch = silent release failure.** Two workflows, two Node
   versions, one test command that only works on one. Fix: KEEP workflow Node versions identical,
   and make the test command portable (`find`, never unquoted `**`). VERIFY THE RELEASE WORKFLOW,
   not just `ci.yml` — a green CI does not mean a published release.
2. **"Green" was asserted from stale snapshots + local runs.** System-reminder stale snapshots and
   async subagent "BUG" reports (recall('') fabrication, Governor NaN, junk-admit, resonance) were
   PRE-FIX views — those bugs were already fixed in v0.3.2. Acting on stale BUG reports wasted cycles.
   Fix: when a subagent claims a bug, verify against CURRENT HEAD, not the report's framing.
3. **Shared `livingMemory()` singleton = flaky/false failures.** Seed-corpus VSA collisions +
   stale `/tmp/bebop-*.json` memory files across runs caused intermittent test-1 failures. The red
   team deterministic commit (0441424) patched two cases but the pattern (global singleton + cross-file
   test ordering) remains fragile. Lesson: make every red-team/integration test HERMETIC — unique
   per-process memory path, no reliance on cross-test global state.
4. **Environment-coupled tests pass locally "by accident."** `harvest.test.ts` hard-coded
   `existingSkills.includes('review')` depending on a skill installed on the dev machine. Fix: inject
   deterministic fixtures; never assert against host-installed packages/skills/files.
5. **Declared "main green" while the user saw red.** The user was looking at the GitHub Releases page
   (Latest = 0.2), not `npm test`. My local 351/0 + CI success blinded me to the release path.
   Fix: before saying "done/green," confirm what the user is actually looking at (Releases? Actions?
   local?) and that the ARTIFACT (release) is published, not just the test command passed.

## WHAT "GREEN" MUST MEAN GOING FORWARD
For this repo a change is only shipped when ALL hold:
- `npm test` (find-based) → 0 fail.
- `pnpm run typecheck` → 0 errors.
- `npm run boot` (Guard-OS self-cert) → certified.
- **BOTH `ci.yml` AND `release.yml` runs green on the pushed commit.**
- **A GitHub Release for the new tag exists and shows as Latest** (the real proof the release path works).

## NEXT SESSION TODO (deferred, not blocked)
- Wire `selectZenoh` (src/integration/zenoh/real-adapter.ts) + `prove` (src/integration/zkvm/prover-adapter.ts)
  into the kernel dispatch (flag-OFF, RED+GREEN proofs) — the "apply findings into real runtime" step
  from docs/design/bleeding-edge-EV-2026-07-08.md.
- Investigate `openwiki-update.yml` workflow-file/permission failure (separate from tests; likely a
  `permissions:` or action-version pin issue). Not a test gate.
- Consider hardening the shared-singleton tests further (hermetic memory per file) to kill residual flakiness.

## FILES CHANGED THIS SESSION (for reference)
- src/consciousness.ts, src/governor.ts, src/knowledge.ts (F6/F7/F8/F9 fixes, v0.3.2)
- src/loop.ts (F10 scope-cwd fix, v0.3.3)
- src/harvest.test.ts (env-decoupling, v0.3.4)
- package.json (`find`-based test), .github/workflows/release.yml (Node 22) (v0.3.5)
- src/integration/redteam-{self,recall,run}.test.ts, zenoh/real-adapter.{ts,test.ts},
  zkvm/prover-adapter.{ts,test.ts} (red-team suites + honest scaffolds)
