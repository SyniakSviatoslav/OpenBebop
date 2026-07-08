// Free-LLM backend — Bebop runs on FREE models by default, no paid key required.
//
// Uses OpenRouter's free-tier models (https://openrouter.ai/models?free=true). All you need is a
// free OpenRouter key (OPENROUTER_API_KEY) — no credit card, no paid plan. Bebop maps its three
// routing lanes to the best currently-free models. If the key is absent, this backend reports
// unavailable and the conductor falls through to other connected agents, finally to the keyless
// native loop — so Bebop ALWAYS boots and runs.
//
// This keeps the promise: "free available LLMs by default, with the ability to connect other
// models or agents (Claude, Codex, OpenCode, ...)".

import type { Model } from './router.ts';

/** Current best-free OpenRouter model per routing lane. Swappable via env if a model is retired. */
const FREE_MODELS: Record<Model, string> = {
  haiku: process.env.BEBOP_FREE_HAIKU ?? 'mistralai/mistral-7b-instruct:free',
  sonnet: process.env.BEBOP_FREE_SONNET ?? 'meta-llama/llama-3.1-8b-instruct:free',
  opus: process.env.BEBOP_FREE_OPUS ?? 'meta-llama/llama-3.1-70b-instruct:free',
};

export interface FreeCallResult {
  ok: boolean;
  text: string;
  model: string;
  reason?: string;
}

export function freeModelFor(lane: Model): string {
  return FREE_MODELS[lane];
}

/** True if a free OpenRouter key is present in the environment. */
export function freeAvailable(): boolean {
  const k = process.env.OPENROUTER_API_KEY ?? process.env.OPENROUTER_FREE_KEY;
  return typeof k === 'string' && k.length > 0;
}

export async function callFreeLLM(lane: Model, prompt: string, system = ''): Promise<FreeCallResult> {
  const key = process.env.OPENROUTER_API_KEY ?? process.env.OPENROUTER_FREE_KEY;
  if (!key) {
    return { ok: false, text: '', model: freeModelFor(lane), reason: 'no OPENROUTER_API_KEY (free key)' };
  }
  const model = freeModelFor(lane);
  const messages = [
    ...(system ? [{ role: 'system', content: system }] : []),
    { role: 'user', content: prompt },
  ];
  try {
    const res = await fetch('https://openrouter.ai/api/v1/chat/completions', {
      method: 'POST',
      headers: {
        Authorization: `Bearer ${key}`,
        'Content-Type': 'application/json',
        'HTTP-Referer': 'https://github.com/SyniakSviatoslav/bebop',
        'X-Title': 'bebop',
      },
      body: JSON.stringify({ model, messages, stream: false }),
    });
    if (!res.ok) {
      const body = await res.text().catch(() => '');
      return { ok: false, text: '', model, reason: `HTTP ${res.status} ${body.slice(0, 160)}` };
    }
    const json = (await res.json()) as any;
    const text = json?.choices?.[0]?.message?.content ?? '';
    return { ok: true, text, model };
  } catch (e: any) {
    return { ok: false, text: '', model, reason: String(e?.message ?? e) };
  }
}
