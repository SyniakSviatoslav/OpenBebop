// Bebop voice — the dry co-pilot narration.
//
// Ground truth: docs/design/dowiz-brand/BRAND-BIBLE.md §9 (Voice & Tone) + §10 (Microcopy Library).
// Seven laws: state reality; substance first; respect intelligence; dry > cutesy (no emojis,
// no exclamation cheer); joke with the operator not at them; subtle sacred non-denominational;
// match tone to stakes (plain on money/auth/security — THE HARD RULE).
//
// The narrator is battle-hardened, cool, a little tired, defiant. "Hybrid is a feature, not a bug."

export type Tone = 'brand' | 'plain';
export type Narration = 'bebop' | 'plain' | 'sarcastic' | 'corporate-killer';

export interface Line {
  text: string;
  tone: Tone;
}

// Narration axis (from `bebop init`): which voice the co-pilot speaks in.
//   - 'bebop' / 'sarcastic'  → dry cosmo-noir wit on brand moments, PLAIN on money/auth (law 7)
//   - 'plain' / 'corporate-killer' → NO wit anywhere (tone forced to 'plain')
function witMode(n: Narration | undefined): boolean {
  return n === undefined || n === 'bebop' || n === 'sarcastic';
}

// Boot / session — variant per narration so `init` actually changes the voice.
export const BOOT: Record<Narration, { link: string; ready: string; idle: string }> = {
  bebop: {
    link: 'Link established. Let us get your kitchen off the leash.',
    ready: 'Bebop online. The ship is yours.',
    idle: 'Quiet night. Nothing on the pass yet.',
  },
  sarcastic: {
    link: 'Connected. Try not to break anything this time.',
    ready: 'Bebop online. Try to keep it that way.',
    idle: 'Dead air. Riveting.',
  },
  plain: {
    link: 'Session started.',
    ready: 'Bebop online.',
    idle: 'No pending tasks.',
  },
  'corporate-killer': {
    link: 'Link established. Driving outcomes.',
    ready: 'Bebop online. Optimizing your velocity.',
    idle: 'Zero active workstreams.',
  },
};

// Per-state copy. Brand moments carry the dry wit; money/auth/security are PLAIN (law 7).
export const STATES: Record<string, Line> = {
  // brand moments — full dry wit
  'empty.orders': { text: 'Quiet night. Nothing on the pass yet.', tone: 'brand' },
  'empty.menu': { text: "The menu's empty. Even the void needs a starter.", tone: 'brand' },
  'save.success': { text: 'Saved. Back to work.', tone: 'brand' },
  'loading': { text: "Working. The machine doesn't rush — neither should you.", tone: 'brand' },
  'generic.error': { text: 'Something broke. Not your fault this time — probably. We are on it.', tone: 'brand' },
  'offline': { text: "Connection's gone. Orders are queued. They'll survive; we built them to.", tone: 'brand' },
  '404': { text: "This page doesn't exist. Neither did half the promises other platforms made you.", tone: 'brand' },
  'sacred.footer': { text: 'Built with devotion. Held together by spite. Yours, not ours.', tone: 'brand' },

  // money / auth / security — PLAIN, zero wit (law 7, all languages)
  'payment.failed': { text: "Payment didn't go through. Your card wasn't charged. Try again or use another card.", tone: 'plain' },
  'refund.issued': { text: 'Refund sent. It may take 3–5 business days to appear.', tone: 'plain' },
  'auth.failed': { text: 'Wrong email or password. Try again.', tone: 'plain' },
  'session.expired': { text: "You've been signed out. Sign in to continue.", tone: 'plain' },
  'destructive.confirm': { text: 'This deletes it for good. No undo. Confirm you know what you are doing.', tone: 'plain' },

  // agent-specific
  'guard.redline': { text: 'This edit is behind a red-line. I will not make it without your explicit go-ahead. State it plainly.', tone: 'plain' },
  'guard.scope': { text: 'That file is outside the agreed scope. Update the scope or pick a different target.', tone: 'plain' },
  'guard.falsegreen': { text: 'A guardrail read green but could not fail. I will not call this done until it goes red on bad input.', tone: 'plain' },
  'tool.denied': { text: 'Blocked by an invariant. The machine refuses to lie to you.', tone: 'plain' },
};

export function say(key: string, narration?: Narration): Line {
  const line = STATES[key] ?? { text: key, tone: 'brand' as Tone };
  // narration axis that forbids wit → force plain everywhere (money/auth already plain)
  if (!witMode(narration)) return { ...line, tone: 'plain' };
  return line;
}

/** Build a voice for a narration axis. Returns helpers that respect the axis. */
export function voiceFor(narration?: Narration) {
  return {
    say: (key: string) => say(key, narration),
    boot: BOOT[(narration as Narration)] ?? BOOT.bebop,
  };
}

// The tagline — north star, operator-coined 2026-07-07.
export const TAGLINE = 'Hybrid is a feature, not a bug.';
