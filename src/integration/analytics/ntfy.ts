/**
 * N8b (2026-07-09): ntfy alert sink — deterministic push of governor early-warnings.
 *
 * The dump's architect question: "how does the system tell me it's degrading 10 min before it
 * fails?" We answer with N7++ `degradationSignal` + `safeState` (deterministic, in-kernel). This
 * module is the DELIVERY seam: when either trips, POST to a self-hosted ntfy.sh topic. It is a
 * thin pure function `notifyNtfy` (and a `shouldAlert` predicate) — NO core I/O, NO runtime
 * dependency on the governor. A caller wires it; the kernel stays air-gapped and pure.
 *
 * Deterministic: given (topic, message) it builds the exact POST (method/url/body/headers) the
 * caller should send. We do NOT open sockets here — keeping the unit testable and the core offline.
 * Falsifiable RED+GREEN: shouldAlert(green) === false; shouldAlert(red) === true; notifyNtfy
 * builds the expected request shape.
 *
 * FLAG-OFF: nothing imports this unless an operator wires an alert topic.
 */

export interface NtfyConfig {
  /** ntfy base URL, e.g. "https://ntfy.sh" or your self-hosted instance. */
  baseUrl: string;
  /** topic name (opaque; treat as a secret — do NOT log it back verbatim). */
  topic: string;
  /** optional title prefix for the push. */
  title?: string;
}

export interface NtfyRequest {
  method: 'POST';
  url: string;
  headers: Record<string, string>;
  body: string;
}

export type AlertKind = 'degradation' | 'safe-state' | 'hallucination-spike' | 'cycle-broken' | 'pca-anomaly';

/** The predicate: alert when the kernel reports a real early-warning, not on healthy ticks. */
export function shouldAlert(s: {
  degradationSignal?: boolean;
  safeState?: boolean;
  hallucinationRate?: number;
  cycleBroken?: boolean;
  pcaAnomaly?: boolean;
}): AlertKind | null {
  if (s.safeState) return 'safe-state';
  if (s.degradationSignal) return 'degradation';
  if (s.hallucinationRate !== undefined && s.hallucinationRate >= 0.5) return 'hallucination-spike';
  if (s.cycleBroken) return 'cycle-broken';
  if (s.pcaAnomaly) return 'pca-anomaly';
  return null;
}

/** Build the exact ntfy POST the caller should dispatch. Pure — caller does the fetch. */
export function notifyNtfy(cfg: NtfyConfig, kind: AlertKind, detail: string): NtfyRequest {
  const body = `[bebop-governor] ${kind}: ${detail}`;
  return {
    method: 'POST',
    url: `${cfg.baseUrl.replace(/\/$/, '')}/${cfg.topic}`,
    headers: {
      'Content-Type': 'text/plain',
      ...(cfg.title ? { Title: cfg.title } : {}),
      Tags: kind, // ntfy emoji tags
    },
    body,
  };
}

/**
 * Convenience: given a governor state, return the ntfy request if an alert is warranted, else null.
 * Keeps the caller to one call: `const req = governorAlertNtfy(cfg, gov.state); if (req) fetch(...)`.
 */
export function governorAlertNtfy(
  cfg: NtfyConfig,
  s: { degradationSignal?: boolean; safeState?: boolean; hallucinationRate?: number; cycleBroken?: boolean; pcaAnomaly?: boolean },
  detail = '',
): NtfyRequest | null {
  const kind = shouldAlert(s);
  if (!kind) return null;
  return notifyNtfy(cfg, kind, detail || JSON.stringify({ hr: s.hallucinationRate, safe: s.safeState }));
}
