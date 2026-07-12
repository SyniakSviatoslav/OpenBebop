#!/usr/bin/env node
// logic-gate.mjs — enforces the Global Logic Laws (docs/design/LOGIC-LAWS.md)
// as truth gates on documentation/claim statements.
//
// Exit codes (see LOGIC-LAWS.md §0):
//   0 = all claims grounded, no contradiction            -> commit allowed
//   1 = HARD logical violation (contradiction / deleted   -> commit REFUSED
//       canonical component)
//   2 = claim needs human arbiter (unbacked / paradox /   -> commit allowed,
//       LEM-assumed)                                          ESC entry written
//
// Honesty: full contradiction/paradox *detection* is undecidable in general.
// This gate enforces the PROCESS (every claim needs a ground; clear
// contradictions are blocked; unprovable/paradox routed to a human) — it does
// NOT claim to prove logic. Limits are documented in LOGIC-LAWS.md §8.

import { execFileSync } from 'node:child_process';
import { readdirSync, readFileSync, existsSync, appendFileSync, statSync } from 'node:fs';
import { join } from 'node:path';

const ROOT = execFileSync('git', ['rev-parse', '--show-toplevel'], { encoding: 'utf8' }).trim();

// --- 1. Repository constitution (Law §6): both components must exist ----------
const PROTOCOL = join(ROOT, 'bebop2');
const AGENT = join(ROOT, 'crates', 'bebop');
if (!existsSync(PROTOCOL)) { console.error('✗ HARD: bebop2/ (protocol) missing — violates LOGIC-LAWS §6'); process.exit(1); }
if (!existsSync(AGENT))   { console.error('✗ HARD: crates/bebop/ (agent) missing — violates LOGIC-LAWS §6'); process.exit(1); }

// --- 2. Collect markdown claim files -----------------------------------------
function walk(dir, out = []) {
  for (const e of readdirSync(dir)) {
    if (e === '.git' || e === 'target' || e === 'node_modules') continue;
    const p = join(dir, e);
    const s = statSync(p);
    if (s.isDirectory()) walk(p, out);
    else if (e.endsWith('.md')) out.push(p);
  }
  return out;
}
const FILES = [
  join(ROOT, 'README.md'),
  join(ROOT, 'AGENTS.md'),
  ...walk(join(ROOT, 'docs')),
  ...walk(join(ROOT, 'bebop2')),
].filter((f) => existsSync(f));

// --- 3. Claim + ground + paradox patterns ------------------------------------
const CLAIM_RE = /(verified|proven|satisfies|guarantees|ensures|post-quantum claim|byte-exact|RED[→\-]>GREEN|kills?|eliminat|is (true|correct|secure|safe|canonical)|no (serde|openssl)|claimed|asserts|proves?|sound)/i;
const GROUND_RE = /(\.(rs|mjs|js|json|toml|wasm)|\[[^\]]+\]\(|#\[test\]|test |proof|KAT|ACVP|NIST|per (ARCHITECTURE|RED-TEAM|ROADMAP)|source:|Stanford|Britannica|Aristotle|Leibniz|Wikipedia)/i;
const NEG_RE = /\b(not|never|no longer|does ?n'?t|isn'?t|is not|cannot|eliminated|removed|killed|gone)\b/i;
const POS_RE = /\b(is|does|verified|satisfies|proven|ensures|guarantees|present|enabled)\b/i;
const PARADOX_RE = /\b(this (statement|claim|sentence|doc(ument)?))\b.{0,40}\b(false|true|unprovable|cannot be (proven|verified))\b/i;

let hardFail = false;
let escCount = 0;
const escLines = [];

function rel(p) { return p.replace(ROOT + '/', ''); }

for (const f of FILES) {
  const lines = readFileSync(f, 'utf8').split('\n');
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    if (!CLAIM_RE.test(line)) continue;

    // Grounding check: claim line OR within ±3 lines must carry a ground.
    const window = lines.slice(Math.max(0, i - 3), Math.min(lines.length, i + 4)).join('\n');
    const grounded = GROUND_RE.test(line) || GROUND_RE.test(window);

    // Paradox check (self-referential truth claim).
    if (PARADOX_RE.test(line)) {
      escCount++;
      const id = `ESC-${Date.now().toString(36)}-${escCount}`;
      escLines.push(`## ${id} — ${new Date().toISOString().slice(0, 10)}\n- Claim: "${line.trim().slice(0, 160)}"  (${rel(f)}:${i + 1})\n- Kind: paradox (self-referential truth claim)\n- Status: OPEN\n- Arbiter: operator\n- Resolution: <fill>\n`);
      console.log(`⚠ ESCALATE(paradox): ${rel(f)}:${i + 1} — "${line.trim().slice(0, 80)}…"`);
      continue;
    }

    if (!grounded) {
      escCount++;
      const id = `ESC-${Date.now().toString(36)}-${escCount}`;
      escLines.push(`## ${id} — ${new Date().toISOString().slice(0, 10)}\n- Claim: "${line.trim().slice(0, 160)}"  (${rel(f)}:${i + 1})\n- Kind: unbacked (no test/proof/citation in ±3 lines)\n- Status: OPEN\n- Arbiter: operator\n- Resolution: <fill>\n`);
      console.log(`⚠ ESCALATE(unbacked): ${rel(f)}:${i + 1} — "${line.trim().slice(0, 80)}…"`);
    }
  }

  // Contradiction check (LNC, Law §2): same subject, opposite predicate.
  const claimIdx = lines.map((l, idx) => (CLAIM_RE.test(l) ? idx : -1)).filter((x) => x >= 0);
  for (let a = 0; a < claimIdx.length; a++) {
    for (let b = a + 1; b < claimIdx.length; b++) {
      const la = lines[claimIdx[a]], lb = lines[claimIdx[b]];
      const subjA = (la.match(/\b([A-Z][A-Za-z0-9_/.-]{2,})\b/)||[])[0];
      const subjB = (lb.match(/\b([A-Z][A-Za-z0-9_/.-]{2,})\b/)||[])[0];
      if (!subjA || subjA !== subjB) continue;
      const aNeg = NEG_RE.test(la), bNeg = NEG_RE.test(lb);
      const aPos = POS_RE.test(la), bPos = POS_RE.test(lb);
      if ((aNeg && bPos) || (aPos && bNeg)) {
        console.error(`✗ HARD CONTRADICTION (LNC): ${rel(f)}:${claimIdx[a] + 1} vs :${claimIdx[b] + 1} — subject "${subjA}" asserted both P and ¬P`);
        hardFail = true;
      }
    }
  }
}

// --- 4. Persist escalations ---------------------------------------------------
if (escLines.length) {
  const marker = '<!-- New ESC entries are appended by logic-gate.mjs above this line. -->';
  const path = join(ROOT, 'docs/design/ESCALATIONS.md');
  let body = readFileSync(path, 'utf8');
  body = body.replace(marker, escLines.join('\n') + '\n' + marker);
  writeFileSyncSafe(path, body);
  console.log(`\n📝 ${escLines.length} escalation(s) logged to docs/design/ESCALATIONS.md — human arbiter required.`);
}

if (hardFail) {
  console.error('\n✗ LOGIC-GATE: hard logical violation — commit REFUSED (resolve contradiction first).');
  process.exit(1);
}
if (escCount > 0) {
  console.log(`\n◆ LOGIC-GATE: ${escCount} claim(s) escalated to human arbiter (commit allowed, tracked).`);
  process.exit(2);
}
console.log('✓ LOGIC-GATE: all claims grounded, no contradiction.');
process.exit(0);

function writeFileSyncSafe(p, c) { appendFileSync(p, ''); require('node:fs').writeFileSync(p, c); }
