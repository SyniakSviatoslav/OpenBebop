#!/usr/bin/env node
// Guardrail — VERIFIED-BY-MATH (VbM): every load-bearing test must be FALSIFIABLE.
// Ported from dowiz/scripts/guardrail-falsifiable-proof.mjs (operator standing rule 2026-07-07)
// and adapted to bebop's test topology (proofs are `src/**/*.test.ts`, not shell-listed .mjs).
//
// PRINCIPLE (VbM #3): a proof that cannot fail is a false-positive metric, not a proof. bebop already
// has the DOC-honesty gate (verify-doc-claims.mjs); this is the deeper PROOF-honesty gate — it stops
// "green tests that prove nothing" (only happy-path asserts, no reachable red branch).
//
// WHAT IT ENFORCES, per `src/**/*.test.ts`:
//   • the file makes real assertions (calls assert.*), AND
//   • at least one assertion is NOT a tautology — i.e. it CAN go red when the code is wrong.
// A test file whose only assertions are tautologies (assert(true) / assert.ok(true) / assert.equal(1,1)
// / assert.equal(x,x)) is flagged: it passes no matter what the code does. Ordinary
// assert.equal(computed, expected) checks are falsifiable and pass.
//
// Falsifiable by design: `--self-test` proves it FLAGS a synthetic all-green test and PASSES a red+green one.
//
// Run: node scripts/guardrail-falsifiable-proof.mjs   |   --self-test
import { readFileSync, existsSync } from 'node:fs';
import { execFileSync } from 'node:child_process';
import { join } from 'node:path';

const ROOT = process.cwd();

// The file actually asserts against reality (not a stub).
const RE_ASSERTS = /\bassert\s*\.\s*\w+\s*\(|\bassert\s*\(/;
// TAUTOLOGICAL assertions — cannot fail regardless of the code, i.e. false-positive metrics:
//   assert(true) / assert.ok(true) / assert.equal(1,1) / assert.equal('x','x') / assert.equal(a,a)
// A test whose ONLY assertions are tautologies proves nothing. A normal assert.equal(x, expected)
// against a computed value IS falsifiable (it goes red when x is wrong) and is fine.
const RE_TAUTOLOGY = new RegExp([
  'assert\\s*\\(\\s*true\\s*\\)',
  'assert\\s*\\.\\s*ok\\s*\\(\\s*true\\s*\\)',
  'assert\\s*\\.\\s*(equal|strictEqual|deepEqual|deepStrictEqual)\\s*\\(\\s*(\\d+)\\s*,\\s*\\2\\s*\\)',           // equal(1,1)
  "assert\\s*\\.\\s*(equal|strictEqual|deepEqual|deepStrictEqual)\\s*\\(\\s*(['\"][^'\"]*['\"])\\s*,\\s*\\2\\s*\\)", // equal('x','x')
  'assert\\s*\\.\\s*(equal|strictEqual|deepEqual|deepStrictEqual)\\s*\\(\\s*(\\w+)\\s*,\\s*\\2\\s*\\)',           // equal(a,a)
].join('|'));
// Strip individual assertion calls, then check whether any NON-tautological assertion remains.
const RE_ASSERT_CALL = /\bassert\s*(?:\.\s*\w+\s*)?\([^;]*?\)/g;

// Verdict for one test file's source text.
function judge(src) {
  const hasAsserts = RE_ASSERTS.test(src);
  if (!hasAsserts) return { falsifiable: false, reasons: ['makes no assert.* calls — it proves nothing'] };
  // does the file contain at least one assertion that is NOT a tautology?
  const calls = src.match(RE_ASSERT_CALL) || [];
  const meaningful = calls.some((c) => !RE_TAUTOLOGY.test(c));
  const reasons = [];
  if (!meaningful) reasons.push('every assertion is a tautology (assert(true)/equal(x,x)) — a proof that cannot go red is a false-positive metric');
  return { falsifiable: reasons.length === 0, reasons, hasAsserts, meaningful };
}

// The ground-truth proof list = every test file the runner executes (self-maintaining).
function testFiles() {
  const out = execFileSync('bash', ['-lc', "find src -name '*.test.ts' | sort"], { cwd: ROOT, encoding: 'utf8' });
  return out.split('\n').map((s) => s.trim()).filter(Boolean);
}

function selfTest() {
  const failures = [];
  const ck = (name, ok) => { if (ok) console.log(`  \u2713 ${name}`); else { console.error(`  \u2717 ${name}`); failures.push(name); } };

  const allGreen = `import assert from 'node:assert'; test('works', () => { assert.equal(1, 1); assert.ok(true); });`;
  const noAsserts = `test('smoke', () => { doThing(); /* no assertions */ });`;
  const falsifiable = `import assert from 'node:assert'; test('green', () => assert.equal(f(),1)); test('RED: bad input throws', () => assert.throws(() => f(NaN)));`;
  const realUnit = `import assert from 'node:assert'; test('computes', () => { assert.equal(accentHexFor('custom'), '#FF0000'); });`;

  ck('all-green tautology (equal(1,1)/ok(true)) \u2192 FLAGGED', judge(allGreen).falsifiable === false);
  ck('no assertions at all \u2192 FLAGGED', judge(noAsserts).falsifiable === false);
  ck('red+green (assert.throws) \u2192 PASSES', judge(falsifiable).falsifiable === true);
  ck('real unit assert against a computed value \u2192 PASSES', judge(realUnit).falsifiable === true);
  ck('testFiles() discovers the real suite', testFiles().length >= 20);

  if (failures.length) { console.error(`\n\u2717 guardrail-falsifiable-proof --self-test: ${failures.length} case(s) failed.`); process.exit(1); }
  console.log('\n\u2713 guardrail-falsifiable-proof --self-test: flags tautological/no-assert tests, passes real ones.');
  process.exit(0);
}

if (process.argv.includes('--self-test')) selfTest();

const files = testFiles();
const violations = [];
for (const rel of files) {
  const abs = join(ROOT, rel);
  if (!existsSync(abs)) { violations.push({ rel, reasons: ['listed by find but not readable'] }); continue; }
  const v = judge(readFileSync(abs, 'utf8'));
  if (!v.falsifiable) violations.push({ rel, reasons: v.reasons });
}

if (violations.length) {
  console.error(`\u2717 guardrail-falsifiable-proof: ${violations.length} test file(s) are NOT falsifiable (Verified-by-Math principle 3):`);
  for (const { rel, reasons } of violations) for (const r of reasons) console.error(`  - ${rel}: ${r}`);
  console.error('\nEvery load-bearing test must be able to go RED. A test that cannot fail is a false-positive metric.');
  process.exit(1);
}
console.log(`\u2713 guardrail-falsifiable-proof: all ${files.length} test file(s) are falsifiable (each has a RED/failure-mode assertion).`);
