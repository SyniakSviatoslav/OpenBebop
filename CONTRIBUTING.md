# Contributing to Bebop

Thanks for wanting to make the ship better. Bebop is **AGPL-3.0-or-later** and accepts
contributions under the **Developer Certificate of Origin (DCO)** — every commit must be signed
off. No CLA, no corporate paperwork.

## The one rule

**All commits must be signed off:**

```bash
git commit -s -m "feat: add warp-drive backend adapter"
```

`-s` appends `Signed-off-by: You <you@example.com>`. PRs with unsigned commits are not merged.
See [DCO.md](./DCO.md).

## How to work

1. Fork / branch off `main`.
2. `npm install` (core deps only; `better-auth` is optional — see README).
3. Keep it green:
   - `npm run boot` — guard-OS self-certification must pass.
   - `npm test` — 105 falsifiable tests must pass.
   - `npm run typecheck` — `tsc --noEmit` clean.
4. **Verified-by-Math:** any behavior change needs a deterministic, *falsifiable* test — a RED
   case (fails on bad input) and a GREEN case (passes on good input). A test that can't fail is
   a false-positive metric and does not count.
5. The **pure core** (`kernel`, `guard`, `governor`, `memory`, `torrent`, `store`, `crypto`)
   must stay deterministic: no clock, no RNG, no network, no env inside those modules.

## Good first contributions

- A new `Backend` adapter in `src/backend.ts` (implement the interface; the router picks it).
- A new `MeshTransport` implementation (libp2p / hyperswarm) behind the existing port.
- More falsifiable tests for the guard OS or governor edge cases.

## Code style

- TypeScript, ESM, `.ts` extensions in imports (NodeNext).
- Match the file's existing voice and comment density.
- Prefer pure functions; push IO/clock/RNG to the shell.

## Reporting issues

Open an issue with: what you ran, what you expected, what you got, and (if possible) a
deterministic repro. Red-line / security findings get a 👀 and a fast, public fix.
