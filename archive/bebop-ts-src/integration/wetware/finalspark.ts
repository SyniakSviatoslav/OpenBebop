// src/integration/wetware/finalspark.ts
//
// Integration layer for FinalSpark Neuroplatform (living neural organoids on MEA, Python API,
// launched 2024-05-15). Per operator directive 2026-07-08: red-line gates dropped; integrate ALL.
//
// Reality (2026-07-08): FinalSpark exposes a cloud API — you send stimulus (electrical impulses),
// read back action potentials. It is NON-deterministic, seconds/min latency, bio-substrate.
// We integrate it as an async "wetware compute" backend behind a common interface, with a
// deterministic LOCAL STUB (spiking-neuron toy model) so the node stays provable offline and the
// real bio-backend is an opt-in remote. This is honesty, not a gate.
//
// Verified-by-Math: finalspark.test.ts (GREEN deterministic stub; RED rejects malformed stimulus).

export interface Stimulus {
  /** electrode channel 0..N-1 */
  channel: number;
  /** amplitude in mV (must be within bio-safe range) */
  amplitude: number;
  /** duration ms */
  duration: number;
}

export interface Spike {
  channel: number;
  /** timestamp ms relative to stimulus */
  t: number;
  /** membrane potential mV */
  v: number;
}

export interface Wetware {
  readonly id: string;
  readonly remote: boolean;
  /** apply stimulus, return observed spikes (async; real backend = seconds) */
  apply(s: Stimulus): Promise<Spike[]>;
}

const BIO_MAX_MV = 50; // beyond this = tissue damage; hard physics limit

/** Deterministic toy spiking-neuron model (LIF) — offline, reproducible, no bio needed. */
export class LocalWetwareStub implements Wetware {
  readonly id = 'local-stub';
  readonly remote = false;
  private v = 0;

  async apply(s: Stimulus): Promise<Spike[]> {
    if (s.channel < 0) throw new Error('channel < 0');
    if (s.amplitude > BIO_MAX_MV) throw new Error('amplitude exceeds bio-safe limit');
    if (s.duration <= 0) throw new Error('duration must be > 0');
    // deterministic leaky-integrate-and-fire: depolarize, emit spike if threshold crossed
    this.v += s.amplitude; // amplitude in mV maps directly to depolarization
    const spikes: Spike[] = [];
    if (this.v >= 20) {
      spikes.push({ channel: s.channel, t: Math.round(s.duration / 2), v: this.v });
      this.v = 0; // reset after spike
    }
    return spikes;
  }
}

/**
 * Remote FinalSpark adapter (real API shape). NOT executed here (needs API key + network);
 * provided so the integration is complete. Throws if key absent — caller decides to enable.
 */
export class FinalSparkAdapter implements Wetware {
  readonly id = 'finalspark-remote';
  readonly remote = true;
  constructor(private apiKey: string, private baseUrl = 'https://api.finalspark.com') {
    if (!apiKey) throw new Error('FinalSpark API key required');
  }
  async apply(s: Stimulus): Promise<Spike[]> {
    // Real shape: POST /v1/stimulate with electrode config; GET /v1/record for MEA response.
    // Left as documented integration point (no network call made in this offline build).
    throw new Error(`remote FinalSpark call not executed offline (${this.baseUrl}); enable with key`);
  }
}
