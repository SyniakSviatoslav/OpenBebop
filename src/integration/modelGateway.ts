/**
 * modelGateway.ts — normalized LLM gateway seam (Portkey-method applied, flag-OFF).
 *
 * Portkey-AI/gateway's VALUE is its schema, not its server: a single normalized call surface with
 * virtual keys, a fallback chain across providers, and guardrail gating. We already HAVE the routing
 * brain (router.ts: cheapest-adequate, red-line escalation). This module layers the gateway ABSTRACTION
 * on top: a `VirtualKey` indirection, an ordered `fallback` chain, and a pre-call GUARDRAIL gate that
 * refuses to forward a red-line task to a sub-opus lane. It is a PURE decision/normalization layer —
 * no network, no server. The caller still performs the actual fetch (mirrors the ntfy/Kalman seams).
 *
 * Deterministic, no RNG/SDG. Falsifiable RED+GREEN. FLAG-OFF: nothing routes through this unless a
 * caller builds a `GatewayConfig` and calls `gatewayRoute`.
 */

import { route, enforceRouting, type Model, type TaskClass } from '../router.ts';

/** A virtual key: an opaque alias for a (provider, credential) pair — never the raw secret in code. */
export interface VirtualKey {
  id: string; // e.g. "vk-opus-prod" — referenced by config, resolved at deploy time
  provider: string; // 'openrouter' | 'anthropic' | 'free-llm' ...
  model: Model; // the lane this key fronts
}

export interface GatewayConfig {
  /** virtual keys indexed by lane; the gateway resolves the right key per routed model. */
  keys: Partial<Record<Model, VirtualKey>>;
  /** ordered fallback chain of lane-models tried if the primary is unavailable. */
  fallback?: Model[];
  /** if true, the guardrail gate refuses to forward red-line tasks to non-opus lanes. */
  guardrails?: boolean;
}

export interface GatewayRequest {
  taskClass: TaskClass;
  /** optional explicit model override (e.g. circuit-breaker forcing a cheaper lane). */
  forceModel?: Model;
}

export interface GatewayPlan {
  ok: boolean;
  /** the primary (model, virtualKeyId) the gateway would forward to. */
  primary?: { model: Model; keyId: string };
  /** ordered fallback (model, keyId) list, applied if primary fails. */
  fallback: { model: Model; keyId: string }[];
  note: string;
}

function keyFor(cfg: GatewayConfig, m: Model): VirtualKey | undefined {
  return cfg.keys[m];
}

/**
 * Resolve a gateway plan for a request: route the task (router.ts), apply the guardrail gate, then
 * build the primary + fallback (model, keyId) chain. RED when guardrails are on and a red-line task
 * resolves to a non-opus lane (the kernel MUST NOT forward money/auth to haiku). GREEN when a normal
 * task resolves to its cheapest-adequate lane with a valid key.
 */
export function gatewayRoute(req: GatewayRequest, cfg: GatewayConfig): GatewayPlan {
  const chosen: Model = req.forceModel ?? route(req.taskClass).model;
  if (cfg.guardrails) {
    const gate = enforceRouting(req.taskClass, chosen);
    if (!gate.ok) {
      return { ok: false, fallback: [], note: gate.note }; // refuse to forward — fail-closed
    }
  }
  const primaryKey = keyFor(cfg, chosen);
  if (!primaryKey) {
    return { ok: false, fallback: [], note: `no virtual key configured for lane ${chosen}` };
  }
  const seen = new Set<Model>([chosen]);
  const fallback: { model: Model; keyId: string }[] = [];
  for (const m of cfg.fallback ?? []) {
    if (seen.has(m)) continue;
    const k = keyFor(cfg, m);
    if (!k) continue; // skip lanes with no key — never fabricate one
    seen.add(m);
    fallback.push({ model: m, keyId: k.id });
  }
  return {
    ok: true,
    primary: { model: chosen, keyId: primaryKey.id },
    fallback,
    note: `gateway plan: ${chosen} via ${primaryKey.id}${fallback.length ? `, fallback: ${fallback.map((f) => f.model).join('>')}` : ''}`,
  };
}
