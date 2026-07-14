#!/usr/bin/env node
// Guardrail — VERIFIED-BY-MATH (VbM): every load-bearing test must be FALSIFIABLE.
// Ported from dowiz/scripts/guardrail-falsifiable-proof.mjs (operator standing rule 2026-07-07)
// and adapted to bebop's NATIVE RUST topology (proofs are `#[test]` fns in crates/bebop + rust-core,
// not TS test files).
//
// PRINCIPLE (VbM #3): a proof that cannot fail is a false-positive metric, not a proof. bebop already
// has the DOC-honesty gate (verify-doc-claims.mjs); this is the deeper PROOF-honesty gate — it stops
// "green tests that prove nothing" (only happy-path asserts, no reachable red branch).
//
// WHAT IT ENFORCES, per `#[test]` fn in the Rust crates:
//   • the test makes real assertions (assert! / assert_eq! / assert_ne!), AND
//   • at least one assertion is NOT a tautology — i.e. it CAN go red when the code is wrong.
// A test whose only assertions are tautologies (assert!(true) / assert_eq!(1,1)) is flagged.
//
// Falsifiable by design: `--self-test` proves it FLAGS a synthetic all-green test and PASSES a red+green one.
//
// Run: node scripts/guardrail-falsifiable-proof.mjs   |   --self-test
import { readFileSync, existsSync } from 'node:fs';
import { execFileSync } from 'node:child_process';
import { join } from 'node:path';

const ROOT = process.cwd();

// A real test makes at least one assertion macro call.
const RE_ASSERTS = /#\[(test|tokio::test)\]|assert!\(|assert_eq!\(|assert_ne!\(/;
// TAUTOLOGICAL assertions — cannot fail regardless of the code:
//   assert!(true) / assert_eq!(1, 1) / assert_eq!(x, x) / assert_ne!(1, 2) (always-true)
const RE_TAUTOLOGY = new RegExp([
  'assert!\\s*\\(\\s*true\\s*\\)',
  'assert_eq!\\s*\\(\\s*(\\d+)\\s*,\\s*\\1\\s*\\)',
  "assert_eq!\\s*\\(\\s*(['\"][^'\"]*['\"])\\s*,\\s*\\1\\s*\\)",
  'assert_eq!\\s*\\(\\s*(\\w+)\\s*,\\s*\\1\\s*\\)',
  'assert_ne!\\s*\\(\\s*(\\d+)\\s*,\\s*(?:\\d+)\\s*\\)',
].join('|'));
const RE_ASSERT_CALL = /#\s*\[(?:tokio::)?test\][\s\S]*?\n\}/g;

// Split a Rust source file into individual #[test] fn bodies, then judge each.
// test body don't truncate the captured body before its assertions.
// Skip a Rust string/char/raw-string literal starting at index `i`.
// `i` points at the opening `r`/`b`/`"`/`'`. Returns the index just past the literal.
// Handles escapes AND raw strings (r"..." / r#"..."# / r##"..."##), plus b"...".
function skipLiteral(src, i) {
  // Raw / byte string prefix: r b br rb (then optional #* then a ").
  if (src[i] === 'r' || src[i] === 'b') {
    let p = i + 1;
    let hashes = '';
    while (src[p] === '#') { hashes += '#'; p++; }
    if (src[p] === '"') {
      const close = '"' + hashes;
      let q = p + 1;
      while (q < src.length) {
        if (src.startsWith(close, q)) return q + close.length;
        q++;
      }
      return src.length;
    }
    // ordinary r"..." / b"..." — the quote is at p
    if (src[p] === '"') i = p;
    else return i; // e.g. identifier like `result` — not a literal
  }
  const q = src[i];
  if (q !== '"' && q !== "'") return i;
  let p = i + 1;
  while (p < src.length) {
    if (src[p] === '\\') { p += 2; continue; }
    if (src[p] === q) return p + 1;
    p++;
  }
  return src.length;
}

function testFns(src) {
  const fns = [];
  // Anchor `#[test]` to a line start (`^\s*#`) so the literal text `#[test]`
  // appearing *inside* a `//!` doc comment is NOT mistaken for a real test
  // attribute — otherwise the next `fn` would be mis-captured as a test body
  // and flagged (false "makes no assertions" violation).
  const re = /^\s*#\s*\[(?:tokio::)?test\][ \t]*[\s\S]*?fn\s+\w+\s*\([^)]*\)\s*\{/gm;
  let m;
  while ((m = re.exec(src)) !== null) {
    let i = m.index + m[0].length; // position right after the opening '{'
    let depth = 1;
    const start = i;
    while (i < src.length && depth > 0) {
      const c = src[i];
      if (c === '{') depth++;
      else if (c === '}') depth--;
      else if (c === '"' || c === "'" || c === 'r' || c === 'b') {
        // skip string/char/raw-string literals so braces inside them don't count
        const adv = skipLiteral(src, i);
        if (adv > i) { i = adv; continue; }
        // not a literal (e.g. identifier starting with r/b) — fall through
      } else if (c === '/' && src[i + 1] === '/') {
        while (i < src.length && src[i] !== '\n') i++;
        continue;
      } else if (c === '/' && src[i + 1] === '*') {
        i += 2;
        while (i < src.length && !(src[i] === '*' && src[i + 1] === '/')) i++;
        i += 2;
        continue;
      }
      i++;
    }
    fns.push(src.slice(start, i - 1));
  }
  return fns;
}

function judge(body) {
  const hasAsserts = /assert!\(|assert_eq!\(|assert_ne!\(/.test(body);
  if (!hasAsserts) return { falsifiable: false, reasons: ['makes no assertion macros — it proves nothing'] };
  const calls = body.match(/assert_(?:eq|ne)!\s*\([^;]*?\)|assert!\s*\([^;]*?\)/g) || [];
  const meaningful = calls.some((c) => !RE_TAUTOLOGY.test(c));
  const reasons = [];
  if (!meaningful) reasons.push('every assertion is a tautology (assert!(true)/assert_eq!(x,x)) — a proof that cannot go red is a false-positive metric');
  return { falsifiable: reasons.length === 0, reasons };
}

function rustFiles() {
  const out = execFileSync('bash', ['-lc', "find crates rust-core -name '*.rs' -not -path '*/target/*' | sort"], { cwd: ROOT, encoding: 'utf8' });
  return out.split('\n').map((s) => s.trim()).filter(Boolean);
}

function selfTest() {
  const failures = [];
  const ck = (name, ok) => { if (ok) console.log(`  ✓ ${name}`); else { console.error(`  ✗ ${name}`); failures.push(name); } };

  const allGreen = `#[test] fn works() { assert!(true); assert_eq!(1, 1); }`;
  const noAsserts = `#[test] fn smoke() { do_thing(); /* no assertions */ }`;
  const falsifiable = `#[test] fn red_green() { assert_eq!(f(), 1); assert!(f_nan(f64::NAN)); }`;

  ck('all-green tautology (assert!(true)/assert_eq!(1,1)) → FLAGGED', judge(allGreen).falsifiable === false);
  ck('no assertions at all → FLAGGED', judge(noAsserts).falsifiable === false);
  ck('red+green (assert_eq computed / assert! predicate) → PASSES', judge(falsifiable).falsifiable === true);
  ck('rustFiles() discovers the real suite', rustFiles().length >= 10);

  if (failures.length) { console.error(`\n✗ guardrail-falsifiable-proof --self-test: ${failures.length} case(s) failed.`); process.exit(1); }
  console.log('\n✓ guardrail-falsifiable-proof --self-test: flags tautological/no-assert tests, passes real ones.');
  process.exit(0);
}

if (process.argv.includes('--self-test')) selfTest();

const files = rustFiles();
let total = 0;
const violations = [];
for (const rel of files) {
  const abs = join(ROOT, rel);
  if (!existsSync(abs)) continue;
  const src = readFileSync(abs, 'utf8');
  for (const body of testFns(src)) {
    total++;
    const v = judge(body);
    if (!v.falsifiable) violations.push({ rel, reasons: v.reasons });
  }
}

if (violations.length) {
  console.error(`✗ guardrail-falsifiable-proof: ${violations.length} of ${total} test fn(s) are NOT falsifiable (Verified-by-Math principle 3):`);
  for (const { rel, reasons } of violations) for (const r of reasons) console.error(`  - ${rel}: ${r}`);
  console.error('\nEvery load-bearing test must be able to go RED. A test that cannot fail is a false-positive metric.');
  process.exit(1);
}
console.log(`✓ guardrail-falsifiable-proof: all ${total} Rust #[test] fn(s) are falsifiable (each has a non-tautological assertion).`);
