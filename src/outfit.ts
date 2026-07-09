// Bebop OUTFIT (2026-07-09) — the ship's identity, declared as ONE source of truth.
//
// This is the "new outfit from today": the cosmo-noir brand made explicit and versioned. It is NOT
// a UI skin (Bebop is a CLI agent) — it is the canonical identity contract every surface reads from:
// the banner, the voice, the sponsor line, the signal colors. Changing the outfit changes the ship.
//
// Brand canon (ground truth, docs/design/dowiz-brand/BRAND-BIBLE.md):
//   · Warm Cosmo-Noir — Cowboy Bebop × cosmo-gothic × Ukrainian irony.
//   · "Hybrid is a feature, not a bug."
//   · Signal color = ship teal #46B0A4 (success / alive / data-signal).
//   · Bone text #F2E9DB on the warm-noir void #12100E.
// This module is the single place that names those tokens so docs/README/wiki can't drift.

export interface Outfit {
  /** semver of the identity itself — bump when the brand contract changes. */
  version: string;
  /** when this identity took the helm (the "outfit" epoch). */
  era: string;
  name: string;
  tagline: string;
  /** the operator's one-liner that defines the whole posture. */
  creed: string;
  /** WCAG-safe palette (paired labels, never color-only). */
  palette: { teal: string; bone: string; void: string; amber: string; blood: string };
  /** the ship mark — one saturated accent, like the brand's "one meaningful color per view" law. */
  sigil: string;
  /** voice axis (see src/voice.ts). */
  narration: 'bebop' | 'plain' | 'sarcastic' | 'corporate-killer';
  /** the dry co-pilot narration lines, per state. */
  lines: Record<string, string>;
  /** where the ship lives / how to reach the crew. */
  home: string;
}

export const OUTFIT: Outfit = {
  version: '1.0.0',
  era: '2026-07-09 — the new outfit',
  name: 'Bebop',
  tagline: 'your kitchen, your ship, your cut.',
  creed: 'Hybrid is a feature, not a bug.',
  palette: { teal: '#46B0A4', bone: '#F2E9DB', void: '#12100E', amber: '#E8A544', blood: '#E0543E' },
  sigil: '◈', // ◈ — cold teal diamond, the machine's eye
  narration: 'bebop',
  lines: {
    boot: 'Bebop online. The ship is yours.',
    multipilot: 'Multipilot engaged — the crew is arguing, the synthesizer decides.',
    field: 'Field arbiter armed: physics gets a veto now.',
    outfit: 'New outfit today. Same ship, sharper cut.',
  },
  home: 'https://github.com/SyniakSviatoslav/bebop',
};

/** Render the outfit banner (used by `bebop outfit` and the launch splash). */
export function outfitBanner(o: Outfit = OUTFIT): string {
  return `${o.sigil} ${o.name} v${o.version} — ${o.tagline}\n   ${o.creed}\n   palette: teal ${o.palette.teal} · bone ${o.palette.bone} · void ${o.palette.void}`;
}
