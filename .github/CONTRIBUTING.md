# Contributing to Bebop

Thanks for wanting to make the ship better. Bebop is **AGPL-3.0-or-later** and accepts
contributions under the **Developer Certificate of Origin (DCO)** — every commit must be signed
off (`git commit -s`). No CLA, no corporate paperwork.

## Quick start for collaborators

```bash
git clone https://github.com/SyniakSviatoslav/bebop.git
cd bebop
npm install
npm run boot     # guard-OS self-certification (must pass)
npm test         # 105 falsifiable tests
npm run typecheck
```

Fork → branch → PR. The CI runs `boot`, `test`, and `typecheck` on every PR.

## The one rule

**All commits must be signed off:** `git commit -s -m "feat: ..."`. Unsigned commits are not
merged. See `DCO.md`.

## Keep it green

- `npm run boot` — the guard OS must certify itself.
- `npm test` — 105 RED+GREEN falsifiable tests.
- `npm run typecheck` — `tsc --noEmit` clean.
- **Verified-by-Math:** every behavior change needs a deterministic, falsifiable test (a RED
  case that fails on bad input + a GREEN case that passes on good input). A test that can't
  fail is a false-positive metric and doesn't count.
- The **pure core** (`kernel`, `guard`, `governor`, `memory`, `torrent`, `store`, `crypto`)
  must stay deterministic: no clock, no RNG, no network, no env inside the decision path.

## Good first contributions

- A new `Backend` adapter (implement the interface in `src/backend.ts`).
- A new `MeshTransport` implementation (libp2p / hyperswarm) behind the existing port.
- More falsifiable tests for the guard OS or governor edge cases.

## Code style

TypeScript, ESM, `.ts` extensions in imports (NodeNext). Match the file's voice. Prefer pure
functions; push IO/clock/RNG to the shell.
