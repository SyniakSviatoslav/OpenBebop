# Constant Doubt — the universal verification rule for Bebop

> **No verification → no statement. Zero guesses. Every claim is falsifiable or it is removed.**

This is the load-bearing law of the Bebop project. It overrides convenience, optimism, and
"it probably works." It applies to **all** prose: README, every `docs/**` page, CHANGELOG,
code comments that describe behavior, and anything we tell a user.

## The rule, in one line

> A statement about Bebop is allowed to exist **only if** a real, reproducible probe or a
> deterministic test backs it. If you cannot make it go RED on bad input, it is not verified.

## What "verified" means

A claim is verified only when **one** of these holds, and the proof command is recorded next to
the claim:

1. **Live probe** — the actual `bebop` binary was run and produced the stated output. The exact
   command is pasted (e.g. `bebop dispatch "edit packages/db/migrations/x.sql"` → `⛔ DENIED`).
2. **Deterministic test** — a test in `npm test` (or `cargo test -p bebop-core`) asserts it, and
   that test has a RED case that flips it. A test that cannot fail is a *false-green metric* and
   does **not** verify anything.
3. **Source of truth** — the claim is a direct quote of code that is itself covered by (1) or (2).

## The three refusals

- **Refuse to state what you haven't run.** If a feature isn't executed, write "not yet verified"
  or nothing — never "works."
- **Refuse to guess at numbers.** Test counts, latencies, model names come from `npm test` output
  or live runs — never memory.
- **Refuse silent drift.** When code changes, the doc that describes it is updated in the same
  breath. A doc that lags the code is a lie.

## How to apply it (checklist before any commit touches docs)

- [ ] Every command named in a doc was actually invoked in this session.
- [ ] Every test file referenced (`*.test.ts`) exists and is in the green suite.
- [ ] Every number (test counts, model routing, globs) was read from live output or source.
- [ ] Every claim about security (what `bebop.json` / `~/.bebop/settings.json` may set) matches
      `src/settings.ts` — the untrusted-project / trusted-user split is law.
- [ ] The RED case ships beside the GREEN case.

## The standing posture: constant doubt

Treat every prior claim — including ones you wrote — as **suspect until re-probed**. The guard
kernel's own `selfTest()` is the model: certify by proving it denies the bad case, not by
asserting it permits the good one. Same for docs.

If you find a doc statement that does not survive a live probe, **fix the doc to match reality**
(or fix the code and re-probe). Never paper over a gap with a confident sentence.

## Standing rule: docs are regenerated and double-checked on EVERY main release

> **A `main` push or tag is not "released" until the documentation pipeline has run and passed.**

Documentation is a first-class artifact of a release, not an afterthought. The moment code lands on
`main` (or a version tag is cut), the docs MUST be brought back in sync and verified with the same
rigor as code:

- **Regenerate, don't hand-patch.** Run `bebop docs build` (typecheck + tests + wasm + diagrams +
  map + i18n parity) so the visuals, counts, and translations reflect the new code — not stale prose.
- **Double-check before the tag.** `bebop docs check` must exit 0: every embedded GIF resolves, the
  machine-readable manifests parse, the version is semver, and OpenWiki + its CI are wired. A
  non-zero exit is a release blocker — fix it, don't tag over it.
- **Mirror to the wiki.** The in-repo wiki (`docs/features/*`, `docs/integrations/*`, `CHANGELOG.md`)
  is the source of truth for agents; update it in the same breath as the release.
- **Same manner, every time.** The procedure is codified as the `release-docs` skill
  (`.bebop/skills/release-docs/SKILL.md`) and the `bebop docs` command. Future releases reuse it
  verbatim — no ad-hoc doc sprints. Storytelling + real recordings + visualizing + translatable +
  parseable + structured, every time.

This rule exists because doc drift is silent: a README that lags the code reads as confident and is
worse than no doc. Constant Doubt applies to the release itself — if the docs weren't re-run, the
release isn't verified.

## Standing rule: better less than sorry — never state what isn't fact-checked

> **If a claim is not backed by a live probe or a deterministic test, it does not appear — anywhere.
> A missing sentence is cheaper than a misleading one.**

This is the harsh corollary of Constant Doubt, and it overrides enthusiasm, marketing, and "it
probably works":

- **No unproven superlatives.** "Best", "unique", "the only tool that…", "production-grade" — banned
  unless a probe or test proves the comparison. Prefer "Bebop does X (verified by `…`)" over
  "Bebop is the best at X".
- **No hidden limitations.** If a feature is partial, say so in the same breath as the claim. A
  feature that "works" only in a TTY, or only with a key, or only on one backend, is documented as
  such — not presented as universal.
- **No fabricated evidence.** A GIF that wasn't a real recording, a test count not read from live
  output, a benchmark not actually run — these are false-greens and are removed on sight.
- **Humble over bold.** When uncertain, write "not yet verified" or "limited to…" rather than a
  confident absolute. The reader trusts the tool more when it admits what it doesn't do.
- **Others have real strengths.** Comparisons credit competing tools where they lead. Bebop is a
  *combiner* above other agentic CLIs, not a replacement; say so plainly.

When in doubt, cut the sentence. A shorter, fully-true doc beats a longer one with one lie.

## Standing rule: doc-claim self-correction layer — claims are machine-checked, never trusted

> **No doc statement reaches a commit or a release unless `scripts/verify-doc-claims.mjs` passes.**
> Every load-bearing claim in README/docs is turned into a falsifiable check that runs the real test
> suite / greps the real source. If the claim can't be proven against live code, the build is RED.

This layer exists because falsified doc statements (animation that wasn't recorded, customization that
was dead, test counts that drifted, ✅/❌ comparison scorecards) shipped repeatedly. Human review alone
did not catch them. So the check is automated and in the path:

- **`scripts/verify-doc-claims.mjs`** — the verifier. Checks: recorder does not force `NO_ANIM=1`
  (animation must be real), launch animation is wired + TTY-gated, customization is read by `settings.ts`
  and tested, PSQ identity has a real test, `recall` returns real payloads, README's test count matches
  `npm test`, no ✅/❌ superiority matrix vs competitors, wiki claim is honest.
- **`bebop docs check`** runs it inside the release-readiness audit — a non-zero verifier status blocks release.
- **`bebop docs build`** runs it in the pipeline.
- **`.git/hooks/pre-commit`** runs it on EVERY commit — a doc claim not backed by live proof refuses the commit.

Adding a new doc claim? Add the matching check to the verifier (with a RED case) in the same change.
Never weaken the verifier to make a doc pass — fix the doc or the code.

## Standing rule: newest plan/decision outranks older (operator, 2026-07-12)

> **When an older roadmap/blueprint and a newer approved decision conflict, the NEWEST wins.**
> MANIFESTO.md + DECISIONS.md (dated 2026-07-12) are the current source of truth and SUPERSEDE
> any 2026-07-11 ROADMAP-GROUND-TRUTH, MASTER-BUILD-SEQUENCE, or stale blueprint citation.

Rationale: plans drift within a session as the operator refines intent (e.g. mesh machinery was
deferred under C8, then pulled forward via D6 operator override; transport was reselected to
DTN/BPv7 + QUIC/BIBE after a deeper research pass showed libp2p was wrong). A stale plan must
never silently overwrite a newer decision.

- Before acting on a roadmap tier/sequence, check the date + the D-series decisions. If a conflict
  exists, follow the newer artifact.
- Record DONE (verified on disk via `grep`/`git`/`cargo test`) separately from PLANNED (desired),
  per the existing ground-truth discipline — but the *precedence* of which plan is live is set by date.
- This rule is mirrored in dowiz DECISIONS.md (D8) and the agent's operating memory.

## Standing setting: newest approved plan/decision IS the source of truth (precedence = setting, not just rule)
> This is a **first-class setting** of both projects, equivalent to a red-line invariant, not a
> soft guideline. It is recorded here as a *setting* so it is discoverable alongside other settings
> (not buried in prose) and cannot be overridden by a stale roadmap.

- **Precedence order (highest → lowest):** operator's latest explicit message in-session
  > newest dated DECISIONS.md / MANIFESTO.md entry > older D-series > 2026-07-11 ROADMAP-GROUND-TRUTH
  > any `MASTER-BUILD-SEQUENCE` / stale blueprint citation.
- **Mesh + post-quantum are NOT deferred.** D6 (operator override of C8) pulls mesh machinery and the
  full PQ protocol into in-scope NOW. C8 still gates *over-engineering*, never the six invariants
  (decentralized · local-first · post-quantum · crypto · mesh · reliability-over-latency).
- **Anu QRNG is wired in**, but native OS entropy is the DEFAULT and FALLBACK (dowiz DECISIONS D9).
  Remote quantum noise upgrades the seed when available; a missing/broken QRNG never breaks boot.
- Any agent operating in either repo MUST re-read DECISIONS.md / MANIFESTO.md (dowiz) and RULES.md +
  this setting (bebop) at session start; a stale plan must never silently overwrite a newer decision.

