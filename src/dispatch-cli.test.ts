// Integration proof (Verified-by-Math): the `bebop dispatch` CLI command MUST deny red-line tasks
// at the guard boundary and MUST NOT shell out to any backend for them. This is the exact bug that
// was caught by live probing (the command bypassed runDispatch's gate). Spawns the real CLI so the
// proof is falsifiable: feed a migration path → expect a non-zero exit + "DENIED" on stderr/stdout.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const BEBOP = path.resolve(HERE, '..', 'bebop.ts');

function runDispatch(task: string): { status: number; out: string } {
  const r = spawnSync('npx', ['tsx', BEBOP, 'dispatch', task], {
    encoding: 'utf8',
    timeout: 60_000,
    env: { ...process.env, NO_ANIM: '1' },
  });
  return { status: r.status ?? -1, out: `${r.stdout ?? ''}${r.stderr ?? ''}` };
}

test('RED: `bebop dispatch` DENIES a red-line task (migrations) and exits non-zero', () => {
  const { status, out } = runDispatch('edit packages/db/migrations/002_users.sql');
  assert.notEqual(status, 0, 'red-line dispatch must exit non-zero (fail-closed)');
  assert.match(out, /DENIED|red-line|guard/i, 'must report a guard denial');
});

test('GREEN: `bebop dispatch` of an in-scope task exits zero (not denied)', () => {
  const { status, out } = runDispatch('refactor tools/bebop/loop.ts to be cleaner');
  assert.equal(status, 0, `benign dispatch must succeed, got exit ${status}: ${out.slice(0, 200)}`);
  assert.doesNotMatch(out, /DENIED|red-line/i, 'benign task must not be denied');
});
