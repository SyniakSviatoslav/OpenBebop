# Guard OS

`src/guard.ts` is the deterministic gate every autonomous action passes through **before** it
runs. It is the spine of Bebop's safety model and the reason the project can claim "the machine
refuses to lie."

## What it checks

1. **Red-line check** — a deny-list of globs (`auth`, `money`, `migrations/`, `*secret*`, …). A
   red-line command is refused *unless* it carries a human approval token. **Fail-closed**: if
   the check can't run, the command is denied.
2. **Scope check** — commands are classified (`read` / `write-file` / `exec` / `network` /
   `red-line`) and compared against the session's granted scope. Over-scope = denied.
3. **Certification** — a deterministic self-test (`selfTest()`) that proves the gate actually
   blocks the bad cases. `bebop boot` runs it; if the gate is broken, nothing autonomous runs.

## Why it's trustworthy

- **Pure**: given the same command + scope it always returns the same verdict. No clock, no RNG,
  no network inside the decision path.
- **Self-certifying**: the gate proves its own red/green behavior via `selfTest()`, which
  returns `{ ok, log }`. The CLI prints the log and exits non-zero if `ok` is false.
- **Tested RED+GREEN**: `guard.test.ts` asserts the gate *denies* a red-line command (RED) and
  *passes* an in-scope one (GREEN). A test that can't fail is a false-positive metric.

## Example

```
$ bebop boot
  · gate 'redline-deny' certified: green on good, red on bad.
  · gate 'scope-block' certified: green on good, red on bad.
  ✓ Bebop guard OS certified: gates deny on red, pass on green.
```

## Extending the guard OS

Add a red-line glob to the deny-list, or a new scope class, in `guard.ts`. Every change must keep
`selfTest()` green — the gate's own proof is the regression test.
