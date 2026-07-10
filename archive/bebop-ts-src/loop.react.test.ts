import { test } from 'node:test';
import assert from 'node:assert/strict';
import { runLoop, reactIters, evalStep, type BebopConfig } from './loop.ts';

// A reusable LLM stub that emits one edit then done, respecting the messages it's given
// (so it can SEE a prior denial and rewrite — the ReAct observe→reflect→reason loop).
function editThenDone(file: string): BebopConfig['llm'] {
  let n = 0;
  return async (messages: any[]) => {
    n++;
    const sawDenial = messages.some((m) => typeof m.content === 'string' && m.content.includes('denied'));
    if (sawDenial && n === 1) {
      // first attempt hit a red-line; on the next call it must have rewritten (proven by caller)
    }
    if (n === 1) return { content: `draft v1 of ${file}`, tool_calls: [{ name: 'edit', args: { path: file, content: 'x' } }] };
    return { content: `done after ${n} iters`, tool_calls: [{ name: 'done', args: {} }] };
  };
}

test('GREEN: default ReAct iterations == 3 (not hidden, visible in result)', async () => {
  const res = await runLoop({ cwd: process.cwd(), taskClass: 'doer', llm: editThenDone('a.ts') });
  assert.equal(res.iterations, 3, 'default must be 3 visible iterations');
  // every iteration is recorded in reactTrace (visible), not collapsed into one "perfect" pass
  assert.ok(res.reactTrace.length >= 3, `reactTrace must expose >=3 steps, got ${res.reactTrace.length}`);
  // at least one reflect phase recorded a real-time eval verdict
  assert.ok(res.reactTrace.some((s) => s.phase === 'reflect' && typeof s.evalScore === 'number'),
    'reflect phase must record a real-time eval score');
});

test('GREEN: iteration count is configurable via cfg.iterations', async () => {
  const res = await runLoop({ cwd: process.cwd(), taskClass: 'doer', iterations: 5, llm: editThenDone('a.ts') });
  assert.equal(res.iterations, 5, 'cfg.iterations must be honored');
});

test('GREEN: iteration count is configurable via BEBOP_REACT_ITERS env', async () => {
  const prev = process.env.BEBOP_REACT_ITERS;
  process.env.BEBOP_REACT_ITERS = '7';
  try {
    assert.equal(reactIters({}), 7, 'env must override default');
    const res = await runLoop({ cwd: process.cwd(), taskClass: 'doer', llm: editThenDone('a.ts') });
    assert.equal(res.iterations, 7, 'env must drive the loop');
  } finally {
    if (prev === undefined) delete process.env.BEBOP_REACT_ITERS; else process.env.BEBOP_REACT_ITERS = prev;
  }
});

test('RED: a denied (red-line) action is recorded in reactTrace as a failed reflect, not hidden', async () => {
  // edit a migrations file -> guard denies -> reflection must show FAIL + rewrite note
  let n = 0;
  const llm: BebopConfig['llm'] = async () => {
    n++;
    if (n === 1) return { content: 'try migrations', tool_calls: [{ name: 'edit', args: { path: 'packages/db/migrations/009_x.sql', content: 'drop' } }] };
    return { content: 'abort', tool_calls: [{ name: 'done', args: {} }] };
  };
  const res = await runLoop({ cwd: process.cwd(), taskClass: 'doer', llm, redLines: ['migrations'] });
  assert.equal(res.denied, 1, 'the red-line edit must be denied');
  const deniedReflect = res.reactTrace.find((s) => s.phase === 'reflect' && s.evalPassed === false);
  assert.ok(deniedReflect, 'the denial must be visible in reactTrace as a failed reflect (not hidden)');
  assert.match(deniedReflect!.reflection!, /FAIL/, 'reflection must say FAIL');
  assert.match(deniedReflect!.reflection!, /rewrote draft/, 'reflection must note the rewrite for next iteration');
});

test('GREEN: evalStep is falsifiable — denied mutation scores 0/FAIL, clean edit scores high/PASS', () => {
  const bad = evalStep({ action: 'edit', observation: '✖ edit denied — red-line', denied: true });
  assert.equal(bad.passed, false);
  assert.equal(bad.score, 0);
  const good = evalStep({ action: 'edit', observation: 'written /tmp/x', denied: false, mutated: true });
  assert.equal(good.passed, true);
  assert.ok(good.score >= 0.9, `clean edit should score high, got ${good.score}`);
});
