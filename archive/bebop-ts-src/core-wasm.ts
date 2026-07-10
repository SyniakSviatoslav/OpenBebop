// Loader for the self-contained bebop-core WASM kernel.
//
// Zero-dependency: uses the WebAssembly global + fs to read the artifact shipped at
// `src/bebop_core.wasm`. If the artifact is missing (e.g. someone cloned without building),
// `initCore()` resolves to null and callers must fall back to the TypeScript port in guard.ts.
//
// C-ABI contract (see crates/core/src/lib.rs): functions read their JSON argument from an input
// region in linear memory that the host writes to, and write their JSON/number result into a
// shared RESULT buffer the host reads back via bebop_result_ptr()/bebop_result_len().

import { readFileSync, existsSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

export interface Decision {
  ok: boolean;
  kind: "ok" | "redline" | "scope" | "error";
  reason?: string;
  deny?: boolean;
}

export interface CoreHandle {
  loaded: true;
  decide(target: string, op?: string, extraDeny?: string[], scope?: string[], cwd?: string): Decision;
  embed(text: string, dim?: number): number[];
  similarity(a: number[], b: number[]): number;
  estimateTokens(text: string): number;
  exportLog(): unknown[];
}

let handle: CoreHandle | null = null;

export async function initCore(): Promise<CoreHandle | null> {
  if (handle) return handle;
  const here = dirname(fileURLToPath(import.meta.url));
  const wasmPath = join(here, "bebop_core.wasm");
  if (!existsSync(wasmPath)) return null;
  try {
    const bytes = readFileSync(wasmPath);
    const { instance } = await WebAssembly.instantiate(bytes, {});
    const ex = instance.exports as Record<string, any>;
    const mem = ex.memory as WebAssembly.Memory;
    if (typeof ex.bebop_decide !== "function") return null;

    // Safe write region: past the static data, in the heap. __heap_base is exported as a number.
    const base = (typeof ex.__heap_base === "number" ? ex.__heap_base : 1024) >>> 0;

    const readResult = (): string => {
      const ptr = ex.bebop_result_ptr() as number;
      const len = ex.bebop_result_len() as number;
      return Buffer.from(new Uint8Array(mem.buffer, ptr, len)).toString("utf8");
    };
    const call = (fn: string, args: unknown): string => {
      const payload = Buffer.from(JSON.stringify(args), "utf8");
      if (payload.length + base > mem.buffer.byteLength) mem.grow(1);
      new Uint8Array(mem.buffer).set(payload, base);
      ex[fn](base, payload.length);
      return readResult();
    };

    handle = {
      loaded: true,
      decide(target, op = "edit", extraDeny = [], scope = [], cwd = "") {
        const out = call("bebop_decide", { target, op, extra_deny: extraDeny, scope, cwd });
        try { return JSON.parse(out) as Decision; } catch { return { ok: false, kind: "error", reason: out }; }
      },
      embed(text, dim = 256) {
        const out = call("bebop_embed", { text, dim });
        try { return JSON.parse(out) as number[]; } catch { return []; }
      },
      similarity(a, b) {
        const n = parseFloat(call("bebop_similarity", { a, b }));
        return Number.isFinite(n) ? n : 0;
      },
      estimateTokens(text) {
        const n = parseInt(call("bebop_estimate_tokens", { text }), 10);
        return Number.isFinite(n) ? n : Math.ceil(text.length / 4);
      },
      exportLog() {
        call("bebop_export_log", {});
        try { return JSON.parse(readResult()) as unknown[]; } catch { return []; }
      },
    };
    return handle;
  } catch {
    return null;
  }
}

export function getCore(): CoreHandle | null {
  return handle;
}
