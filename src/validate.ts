// Bebop validate — the PYDANTIC principle applied at the agent's trust boundary.
//
// Reverse-engineered from pydantic (v2): validation is NOT optional nudging, it is the BOUNDARY
// LAYER every external input must clear before it becomes an internal model. pydantic's core idea
// we adopt: a single, explicit, fail-fast contract at the edge; malformed input never reaches the
// engine. We apply it to TOOL ARGUMENTS — the agent's untrusted input from the LLM — before any
// tool runs (and therefore before the guard even sees them). This is the "boundary is a wall, not a
// suggestion" lesson: validation lives at the seam, never scattered through tool bodies.
//
// All checks PURE + DETERMINISTIC. Verified-by-Math: RED+GREEN in src/validate.test.ts.

import type { ToolName } from './loop.ts';

export interface ValidatedArgs {
  ok: true;
  name: ToolName;
  path?: string;
  content?: string;
  cmd?: string;
  task?: string;
  pattern?: string;
}
export interface ValidationFailure {
  ok: false;
  name: ToolName | null;
  reason: string;
}
export type ValidationResult = ValidatedArgs | ValidationFailure;

// The contract: every tool's required fields, types, and shape. Mirrors pydantic field constraints.
type Contract = Record<ToolName, { required: (keyof ValidatedArgs)[] }>;

const CONTRACT: Contract = {
  read: { required: ['path'] },
  grep: { required: ['pattern'] },
  edit: { required: ['path', 'content'] },
  run: { required: ['cmd'] },
  dispatch: { required: ['task'] },
  done: { required: [] },
};

function isNonEmptyString(v: unknown): v is string {
  return typeof v === 'string' && v.length > 0;
}

/**
 * Validate raw tool-call args against the tool's contract. Returns a typed, safe payload or a
 * hard failure. The LLM can send anything — this is the wall. No mutation side-effects; pure.
 */
export function validateToolArgs(name: unknown, args: unknown): ValidationResult {
  if (typeof name !== 'string' || !(name in CONTRACT)) {
    return { ok: false, name: null, reason: `unknown tool '${String(name)}' — not in contract` };
  }
  const tool = name as ToolName;
  const a = (args ?? {}) as Record<string, unknown>;
  for (const field of CONTRACT[tool].required) {
    const v = a[field as string];
    if (!isNonEmptyString(v)) {
      return { ok: false, name: tool, reason: `tool '${tool}' requires non-empty '${field}'` };
    }
  }
  // Pass only known, typed fields forward — drop anything the contract doesn't define.
  const out: ValidatedArgs = { ok: true, name: tool };
  if (isNonEmptyString(a.path)) out.path = a.path;
  if (isNonEmptyString(a.content)) out.content = a.content;
  if (isNonEmptyString(a.cmd)) out.cmd = a.cmd;
  if (isNonEmptyString(a.task)) out.task = a.task;
  if (isNonEmptyString(a.pattern)) out.pattern = a.pattern;
  return out;
}
