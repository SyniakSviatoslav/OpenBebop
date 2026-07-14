# Integration Decart Rule — compare & probe before you adopt (operator, 2026-07-14)

> Standing order for every bebop2 agent/lane. Companion to §0–§4 of `AGENTS.md`. Encodes the operator's
> technology-selection stance: **agnostic, innovative, ethical — zero ideological attachments. Always
> compare & probe.**

## Principle

Decide by **honest, falsifiable, critical comparison** — never by appeal to authority. Modern /
Rust-native is the **default and the tiebreak**; a proven classical/mature method wins **only when an
honest comparison proves it genuinely better on the merits.** No ideological attachment to *either* the
new or the old. (This is the same discipline as §2 VbM — decisions, like tests, are validated only by
falsifiable evidence, not narrative.)

## The rule

Any **new integration** MUST pass a decart evaluation **first**, and leave a **decart comparison report**
in the commit/PR that introduces it. No silent adoption.

**"New integration" =** a new crate in `Cargo.toml` [dependencies] · a new external service / API · a new
transport / provider / carrier / protocol · **or replacing one of these with another.**
**Not** covered: internal refactors, in-line version bumps within a pinned line, dev-only tooling that
never ships in the sovereign core.

## The decart table (comparison report)

One row per criterion, one column per candidate. Cite **evidence** (a KAT name, a number, a link) — not
social proof.

| Criterion | Modern / Rust-native default | Proven / classical alt | (other) |
|---|---|---|---|
| Fit to the sovereign core (`no_std + alloc`, zero-dep, offline-buildable) | | | |
| Correctness & security — *falsifiable* proof (KAT/ACVP, constant-time, verifier-actually-rejects) | | | |
| Performance — *measured*, not assumed | | | |
| Supply-chain & license (`cargo-deny`/`deny.toml` clean; no banned C build — openssl-sys/native-tls — unless justified) | | | |
| Maintainability & clarity (readable, easy to change) | | | |
| Reversibility — can it be a port / adapter / fallback instead of a core commitment? | | | |
| Evidence cited (KAT / number / link) — NOT "everyone uses it" | | | |

**`DECISION: <chosen> — <honest falsifiable reason>.`**
- **Tiebreak:** criteria tie, or the alternative's advantage is unproven → **modern / Rust-native wins.**
- **Older-as-adapter:** if a non-default is chosen, or an older tech is kept alongside, state plainly that
  it is a **bridge / fallback / port — not purged.** (No dogmatic elimination.)
- **Probe (mandatory):** state the **strongest honest argument AGAINST** the decision and why it didn't
  win. If you cannot state one, you have not probed — go back.

## Banned as a *deciding* reason

"Industry standard / more mature / battle-tested / community-approved / everyone uses it." Social proof
and tradition are **not evidence**. (An honest *technical* case for a mature tool is welcome — and if it
wins on the merits, it is chosen. The ban is on using popularity *as the argument*.)

## Worked example (a real decart — the rustls-WSS migration)

**Choice:** TLS crypto provider for the `proto-wire` wss/iroh transport — `rustls + ring` vs `aws-lc-rs`
vs `native-tls (openssl-sys)`.

| Criterion | rustls + ring (chosen) | aws-lc-rs | native-tls / openssl-sys |
|---|---|---|---|
| Sovereign-core fit | pure-Rust provider, offline-buildable default | C library (aws-lc) | C lib + system OpenSSL |
| Correctness proof | `hardened_verifier_rejects_self_signed_cert` (verifier *actually* rejects an untrusted cert) | same rustls API | not exercised |
| Supply-chain / license | in-lock, `cargo-deny` clean | C build, acceptable | drags openssl-sys — deny hard-fails |
| Reversibility | primary provider via `builder_with_provider(ring::default_provider())` | **kept as compiled fallback (bridge)** | rejected |

**DECISION:** `rustls + ring` primary — the Rust-native default, proven by a falsifiable negative test.
**Older-as-adapter:** `aws-lc-rs` stays compiled as an accepted fallback (a bridge, **not purged**);
`native-tls` rejected because it re-introduces the banned openssl-sys C supply chain (§3G/F6), which
`cargo deny` is configured to hard-fail. **Probe:** the honest case *against* was "aws-lc-rs is
FIPS-validated / more battle-tested" — rejected as a *deciding* reason (appeal to authority); no
falsifiable requirement here needs FIPS validation, and ring's correctness is proven by KAT + the
verifier test. Commits c837442 / 405a3a8 / a24127b.

## Enforcement

- **Now (guidance):** this rule is a standing order (`AGENTS.md §5`); every agent applies it before adding
  or swapping an integration, and attaches the decart report to the change.
- **Follow-up (script gate):** a deterministic pre-commit check (a new `[dependencies]` line in a
  `Cargo.toml` ⇒ require a linked decart report) would slot next to the existing gates. Because any new
  gate must itself pass the 3-model review (§1), it is a scoped follow-up; the standing rule is authority
  meanwhile.
