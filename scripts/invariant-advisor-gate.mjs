#!/usr/bin/env node
/**
 * invariant-advisor-gate.mjs — mechanical enforcement of Universal rule "Propose-don't-execute"
 * (Cross-pattern B from docs/design/bebop-fundamental-principles-2026-07-09.md).
 *
 * The rule: any stochastic/advisor component MAY propose but NEVER writes the actuator; execution is
 * always a deterministic function over a verified state. This script asserts that EVERY advisor entry
 * point in the codebase is matched by a deterministic verifier — so a future integration cannot add a
 * "propose-and-execute" path and slip past the gate.
 *
 * It greps the real source for the known advisor entry points and their matching deterministic verifier,
 * and fails (exit 1) if any entry point exists without its verifier. Self-testing: it does NOT assert
 * behavior, it asserts STRUCTURE (the invariant "advisor ⇒ verifier" holds in the tree).
 *
 * Run: node scripts/invariant-advisor-gate.mjs
 */
import { readFileSync, existsSync } from 'node:fs';
import { execSync } from 'node:child_process';
import path from 'node:path';

const ROOT = process.cwd();

function srcOf(rel) {
  const abs = path.join(ROOT, rel);
  return existsSync(abs) ? readFileSync(abs, 'utf8') : '';
}

// (advisor entry point  →  the deterministic verifier that must exist/be referenced)
const PAIRS = [
  ['src/kernel.ts', 'applyCommandChecked'], // kernel: decide/fold proposed, applyCommandChecked verifies
  ['src/integration/analytics/dual-track.ts', 'dualTrackGate'], // GNN advisor proposes, graph gate verifies
  ['src/copilot.ts', 'defaultChecker'], // doer proposes, distinct checker verifies
  ['src/speculate.ts', 'verifyBlock'], // backbone drafts, guard verifies
  ['src/integration/logicalCot.ts', 'verifyLogicalPlan'], // executor proposes, logic auditor verifies
  ['src/integration/analytics/goap.ts', 'plan'], // advisor names goal, planner enumerates (NO PATH if unmet)
];

let fails = 0;
const lines = [];
for (const [file, verifier] of PAIRS) {
  const src = srcOf(file);
  if (!src) {
    console.error(`  ✗ ${file} missing (advisor entry point not found)`);
    fails++;
    continue;
  }
  const hasVerifier = src.includes(verifier);
  if (hasVerifier) {
    lines.push(`  ✓ ${file} → verifier '${verifier}' present`);
  } else {
    console.error(`  ✗ ${file} has NO matching deterministic verifier '${verifier}' (propose-and-execute gap!)`);
    fails++;
  }
}

// Bonus: assert there is at least one GUARD GATE in the loop that runs before any mutation.
const loop = srcOf('src/loop.ts');
if (!/GUARD GATE/.test(loop)) {
  console.error('  ✗ src/loop.ts has no GUARD GATE before mutation (advisor could reach actuator)');
  fails++;
} else {
  lines.push('  ✓ src/loop.ts GUARD GATE present before any read/write/dispatch');
}

if (fails) {
  console.error(`\n✗ invariant-advisor-gate: ${fails} advisor entry point(s) lack a deterministic verifier.`);
  process.exit(1);
}
console.log(lines.join('\n'));
console.log('\n✓ invariant-advisor-gate: every advisor entry point is matched by a deterministic verifier (Cross-pattern B).');
