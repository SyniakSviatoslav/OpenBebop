# Outfit

The **cosmo-noir identity contract** — the "new outfit" (2026-07-09). Declared ONCE in
`crates/bebop/src/outfit.rs` (`OUTFIT` const) so docs/README/wiki can't drift.

- **Name:** Bebop · **Era:** 2026-07-09 — the new outfit · **Identity version:** v1.0.0
- **Persona:** Warm Cosmo-Noir — Cowboy Bebop × cosmo-gothic × Ukrainian irony. Dry, precise.
- **Creed:** *"Hybrid is a feature, not a bug."*

## Palette (WCAG-safe, paired labels — never color-only)
| role | hex | use |
|------|-----|-----|
| ship | `#F4C25A` | the ship — luminous sun-gold, the launch ritual |
| tele | `#F2933E` | telemetry / data-signal — warm orange |
| glow | `#FFD9A0` | luminous peach — loader halo, hints, working-feed spark |
| bone | `#FBF3E7` | primary text on void (warm ivory) |
| void | `#12100E` | the warm-noir ground |
| alert | `#E0543E` | warm red — reserved for alerts / drift / hallucination |

This is the canonical identity every surface reads: banner, voice, sponsor line, signal colors.

## Usage
```bash
bebop outfit        # prints the identity contract
bebop init          # pick narration + looks (they actually change the CLI)
bebop preview       # render the cosmo-noir helm to SVG with your accent
```
