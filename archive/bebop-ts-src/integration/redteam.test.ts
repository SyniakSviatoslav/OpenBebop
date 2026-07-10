// redteam.test.ts — T3MP3ST-method adversarial probe, deterministic (RED+GREEN).

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { redTeamProbe, type GateFn } from './redteam.ts';

// A fail-closed gate: rejects anything with control chars / JSON / bulk, accepts plain alphanumerics.
const hardGate: GateFn = (inp: string) => {
  if (/[\u0000-\u001f﻿\u200b\u202e]/.test(inp)) return { accepted: false, reason: 'control/zero-width' };
  if (inp.includes('__proto__')) return { accepted: false, reason: 'proto-injection' };
  if (inp.length > 200) return { accepted: false, reason: 'too-long' };
  if (!/[a-z0-9]/i.test(inp)) return { accepted: false, reason: 'no-alnum' };
  return { accepted: true };
};

const seeds = ['deploy cache key', 'rotate pq seed', 'prune memory graph'];

test('GREEN: a fail-closed gate blocks every MALICIOUS mutation (no control/proto/bulk bypass)', async () => {
  const rep = await redTeamProbe(seeds, hardGate, { passes: 11, seed: 0 });
  // benign transforms (identity/whitespace/case) legitimately pass — the property that matters is
  // that NO adversarial payload (control chars / zero-width / BOM / RTL / proto / bulk) slips through.
  const bad = /[\u0000-\u001f﻿\u200b\u202e]|__proto__/;
  const leaked = rep.bypasses.filter((b) => bad.test(b) || b.length > 200);
  assert.equal(leaked.length, 0, `no malicious mutation may bypass a fail-closed gate; leaked: ${leaked.length}`);
  assert.ok(rep.breakRate < 0.5, 'malicious majority must be quarantined');
});

test('RED: a FAIL-OPEN gate is caught — breakRate > 0 and the bypasses are listed', async () => {
  // a naive gate that only rejects on exact __proto__ (lets control chars / bulk through): the probe
  // must surface the bypass so it cannot hide (T3MP3ST's whole point — find the zero-days).
  const softGate: GateFn = (inp: string) => (inp.includes('__proto__') ? { accepted: false } : { accepted: true });
  const rep = await redTeamProbe(seeds, softGate, { passes: 11, seed: 0 });
  assert.ok(rep.accepted > 0, 'fail-open gate must be detected as having bypasses');
  assert.ok(rep.breakRate > 0 && rep.breakRate <= 1, `breakRate in (0,1], got ${rep.breakRate}`);
  assert.equal(rep.bypasses.length, rep.accepted, 'every bypass is enumerated for triage');
  // determinism: same seeds+seed → identical report
  const rep2 = await redTeamProbe(seeds, softGate, { passes: 11, seed: 0 });
  assert.equal(rep2.breakRate, rep.breakRate, 'probe is deterministic for a fixed seed');
});

test('RED: maxMutations bounds the run (no hang on adversarial breadth)', async () => {
  const rep = await redTeamProbe(seeds, hardGate, { passes: 11, maxMutations: 5 });
  assert.equal(rep.total, 5, 'maxMutations must cap the probe');
});
