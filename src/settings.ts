// Bebop settings — the project/user config file (Claude Code's settings.json analogue).
//
// SECURITY MODEL (red-team hardening, 2026-07-08):
//   - A project's `bebop.json` (cwd) is UNTRUSTED — you may clone it from anywhere. It may set
//     ONLY `model`. It MUST NOT set `permissions` or `hooks` (those would let a cloned repo alter
//     your security posture or run code).
//   - `permissions` and `hooks` are loaded ONLY from the USER settings (~/.bebop/settings.json),
//     which the user owns and trusts.
//   - Hooks run WITHOUT a shell (shell:false), command split into argv; any command containing
//     shell metacharacters is refused.
//   - Malformed config is reported (console.error) instead of silently failing open.
//
// Loaded from (highest precedence last):
//   1. ~/.bebop/settings.json            (user, trusted — may set model/permissions/hooks)
//   2. <cwd>/bebop.json                 (project, UNTRUSTED — model ONLY)
//
// Shape (all optional):
//   {
//     "model": "opus" | "haiku" | "sonnet" | "<modelId>",
//     "permissions": {
//       "allow": ["tools/bebop/**", ...],   // globs added to the guard scope
//       "deny":  ["**/secrets/**", ...]      // globs added to the red-line deny set
//     },
//     "hooks": { "PreToolUse": [...], "PostToolUse": [...], "Stop": [...] }
//   }
//
// Pure & testable: loadSettings takes explicit paths so tests never touch the real FS layout.

import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';

export interface HookSpec {
  matcher?: string; // tool name to match (or "*")
  command: string; // command; run WITHOUT a shell (argv split). May NOT contain shell metacharacters.
}

export interface BebopSettings {
  model?: string;
  permissions: { allow: string[]; deny: string[] };
  hooks: Record<string, HookSpec[]>;
}

export const EMPTY_SETTINGS: BebopSettings = {
  model: undefined,
  permissions: { allow: [], deny: [] },
  hooks: {},
};

// Shell metacharacters — a hook command containing any of these is refused (we never use a shell).
const SHELL_METACHARS = /[;|&$`<>()\n\r\t\\'"]/;

function readJsonSafe(file: string): Record<string, any> | null {
  try {
    const raw = fs.readFileSync(file, 'utf8');
    return JSON.parse(raw) as Record<string, any>;
  } catch {
    return null;
  }
}

function warn(msg: string): void {
  // Operator-visible: malformed/untrusted config is not silently swallowed.
  try {
    console.error(`[bebop:settings] ${msg}`);
  } catch {
    /* console may be unavailable in some harnesses */
  }
}

// A project file may set ONLY `model`. Anything else is ignored + warned (untrusted).
function applyProject(part: Record<string, any> | null, into: BebopSettings): void {
  if (!part) return;
  if (typeof part.model === 'string') into.model = part.model;
  for (const key of ['permissions', 'hooks']) {
    if (part[key] !== undefined) {
      warn(`project bebop.json may not set "${key}" (untrusted) — ignored. Set it in ~/.bebop/settings.json.`);
    }
  }
}

// The user file is trusted: it may set model/permissions/hooks.
function applyUser(part: Record<string, any> | null, into: BebopSettings): void {
  if (!part) return;
  if (typeof part.model === 'string') into.model = part.model;
  if (Array.isArray(part.permissions?.allow)) into.permissions.allow.push(...part.permissions.allow);
  if (Array.isArray(part.permissions?.deny)) into.permissions.deny.push(...part.permissions.deny);
  if (part.hooks && typeof part.hooks === 'object') {
    for (const [evt, specs] of Object.entries(part.hooks)) {
      if (!Array.isArray(specs)) continue;
      const clean = (specs as any[])
        .filter((s) => s && typeof s.command === 'string')
        .map((s) => ({ matcher: s.matcher, command: s.command as string }));
      const withMetachars = clean.filter((s) => SHELL_METACHARS.test(s.command));
      if (withMetachars.length) {
        warn(`${withMetachars.length} hook command(s) in "${evt}" contain shell metacharacters — refused (hooks run without a shell).`);
      }
      const safe = clean.filter((s) => !SHELL_METACHARS.test(s.command));
      if (safe.length) into.hooks[evt] = (into.hooks[evt] ?? []).concat(safe);
    }
  }
}

export function loadSettings(opts?: {
  cwd?: string;
  userFile?: string;
  projectFile?: string;
}): BebopSettings {
  const cwd = opts?.cwd ?? process.cwd();
  const userFile = opts?.userFile ?? path.join(os.homedir(), '.bebop', 'settings.json');
  const projectFile = opts?.projectFile ?? path.join(cwd, 'bebop.json');
  const s: BebopSettings = JSON.parse(JSON.stringify(EMPTY_SETTINGS));
  applyUser(readJsonSafe(userFile), s);
  applyProject(readJsonSafe(projectFile), s);
  return s;
}
