// shadow.test.ts — shadow-mode composition of the proven-but-flag-OFF seams (RED+GREEN).
// Per Universal rule Flag-OFF → shadow → gate: runs logicalCot + dualTrack + validate NON-BLOCKING.

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { shadowVerify } from './shadow.ts';
import type { ToolName } from '../loop.ts';

test('GREEN: a well-formed, graph-consistent action is shadow-ok on all axes', () => {
  const r = shadowVerify('read' as ToolName, { path: 'src/x.ts' }, {
    graph: { nodes: ['hub', 'p'], A: [[0, 1], [0, 0]] },
    advisor: { propose: () => ({ target: 'p', confidence: 0.9 }) },
    focus: 'hub',
  });
  assert.equal(r.wouldBlock, false);
  assert.equal(r.axes.validate!.ok, true);
});

test('RED: a malformed tool-arg is flagged by the validate axis (shadow only, no block)', () => {
  const r = shadowVerify('edit' as ToolName, { path: '' }, {}); // edit requires non-empty content+path
  assert.equal(r.axes.validate!.ok, false);
  assert.equal(r.wouldBlock, true, 'shadow reports what WOULD block, but does not block');
});

test('GREEN: omitting an axis skips it (caller can shadow just one seam)', () => {
  const r = shadowVerify('grep' as ToolName, { pattern: 'x' }, {});
  assert.ok(!('dualTrack' in (r.axes ?? {})));
  assert.equal(r.axes.validate!.ok, true);
});
