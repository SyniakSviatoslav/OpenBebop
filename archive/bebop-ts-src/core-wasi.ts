// Bebop Sovereign Node — Phase 2: WASI core runtime (WasmEdge).
//
// The deterministic kernel can run TWO ways:
//   1. in-process: WebAssembly.instantiate on bebop_core.wasm (core-wasm.ts, default).
//   2. hardened:    a WasmEdge process running bebop_core.wasi.wasm (this file) — AOT-compiled,
//                   MB-sized, proven-sandbox. Used when BEBOP_CORE_RUNTIME=wasi on the Sovereign Node.
//
// The LLM periphery (Ollama) is NEVER inside the WASI sandbox — inference stays native; only the
// decide/embed/similarity core is hardened. This is the "cheapest token is the one you never send"
// principle: the deterministic core is isolated from any LLM being wrong.

import { spawnSync } from "node:child_process";
import { existsSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

export interface CoreHandle {
  loaded: true;
  decide(target: string, op?: string, extraDeny?: string[], scope?: string[], cwd?: string): {
    ok: boolean; kind: "ok" | "redline" | "scope" | "error"; reason?: string; deny?: boolean;
  };
  embed(text: string, dim?: number): number[];
  similarity(a: number[], b: number[]): number;
  estimateTokens(text: string): number;
  exportLog(): unknown[];
}

function runWasmEdge(wasmPath: string, fn: string, args: unknown): string {
  const res = spawnSync("wasmedge", [wasmPath, fn, JSON.stringify(args)], {
    encoding: "utf8",
    maxBuffer: 1 << 22,
  });
  if (res.status !== 0) {
    throw new Error(`wasmEdge ${fn} failed (${res.status}): ${res.stderr || res.stdout}`);
  }
  return (res.stdout || "").trim();
}

export async function initWasiCore(): Promise<CoreHandle | null> {
  const here = dirname(fileURLToPath(import.meta.url));
  // Prefer the AOT artifact; fall back to the raw wasi.wasm (run via wasmedge interpreter).
  const aot = join(here, "..", "dist", "bebop_core.wasi.aot.wasm");
  const raw = join(here, "..", "dist", "bebop_core.wasi.wasm");
  const wasmPath = existsSync(aot) ? aot : raw;
  if (!existsSync(wasmPath)) return null;
  try {
    // Smoke-test the binary before advertising a handle.
    runWasmEdge(wasmPath, "bebop_decide", { target: "x", op: "edit", extra_deny: [], scope: [], cwd: "" });
  } catch {
    return null;
  }
  const call = (fn: string, args: unknown): string => runWasmEdge(wasmPath, fn, args);
  const parse = <T>(s: string, fb: T): T => {
    try { return JSON.parse(s) as T; } catch { return fb; }
  };
  return {
    loaded: true,
    decide: (target, op = "edit", extraDeny = [], scope = [], cwd = "") =>
      parse(call("bebop_decide", { target, op, extra_deny: extraDeny, scope, cwd }),
        { ok: false, kind: "error" as const }),
    embed: (text, dim = 256) => parse<number[]>(call("bebop_embed", { text, dim }), []),
    similarity: (a, b) => { const n = parseFloat(call("bebop_similarity", { a, b })); return Number.isFinite(n) ? n : 0; },
    estimateTokens: (text) => {
      const n = parseInt(call("bebop_estimate_tokens", { text }), 10);
      return Number.isFinite(n) ? n : Math.ceil(text.length / 4);
    },
    exportLog: () => parse<unknown[]>(call("bebop_export_log", {}), []),
  };
}
