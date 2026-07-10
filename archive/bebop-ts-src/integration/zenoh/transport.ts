// src/integration/zenoh/transport.ts
//
// Integration layer for Eclipse Zenoh (Zero-Overhead Pub/Sub) into the bebop Sovereign Node.
//
// Reality check (2026-07-08): @eclipse-zenoh/zenoh-ts@1.9.0 exists on npm (Rust core + TS
// bindings). Full native integration needs the Zenoh router/binary; in a constrained/offline node
// we provide a DETERMINISTIC in-process implementation of Zenoh's core *semantics* — pub/sub over
// named keys, decentralized peer mesh (gossip), priority-based non-destructive arbitration (lower
// id wins, like CAN bus), and last-value store/query. The real zenoh-ts client drops in behind
// `ZenohTransport` (swap `createLocalMesh` for a `createZenoh` that returns the same interface).
//
// Verified-by-Math: see transport.test.ts (GREEN = correct on good input; RED = rejects bad input).

export interface Envelope {
  /** signed payload; any Uint8Array — reuse core crypto Envelope */
  payload: Uint8Array;
  /** node id that produced this; used for mesh arbitration */
  from: string;
  /** monotonic per-node seq for deterministic ordering */
  seq: number;
  /** priority 0..255 (lower = higher priority, CAN-bus style) */
  priority: number;
  /** key expression, e.g. "l5/telemetry/<node>" */
  key: string;
}

export interface Transport {
  /** publish an envelope on a key; mesh delivers to local subscribers + returns delivery count */
  put(env: Envelope): number;
  /** subscribe to a key prefix; returns unsubscribe */
  subscribe(keyPrefix: string, fn: (env: Envelope) => void): () => void;
  /** query the local store for a key (store/query union, Zenoh-style) */
  get(key: string): Envelope | undefined;
  /** id of this node */
  readonly id: string;
}

const keyMatches = (key: string, prefix: string): boolean => {
  if (prefix === key) return true;
  // Zenoh key expressions are hierarchical "a/b/c"; prefix match on segments
  const k = key.split('/');
  const p = prefix.split('/');
  for (let i = 0; i < p.length; i++) {
    if (p[i] === '**') return true; // wildcard tail
    if (k[i] !== p[i]) return false;
  }
  return true;
};

/** Deterministic priority arbitration: lower priority id wins; tie -> lower from-id string. */
const wins = (a: Envelope, b: Envelope): boolean => {
  if (a.priority !== b.priority) return a.priority < b.priority;
  return a.from < b.from;
};

export class LocalMesh implements Transport {
  readonly id: string;
  private subs = new Map<string, Array<(e: Envelope) => void>>();
  private store = new Map<string, Envelope>(); // last-value store per key
  private peers: LocalMesh[] = [];

  constructor(id: string) {
    if (!id || id.length === 0) throw new Error('node id required');
    this.id = id;
  }

  /** Attach another LocalMesh as a peer (decentralized mesh; no central router). */
  connect(peer: LocalMesh): void {
    if (this.peers.includes(peer) || peer === this) return;
    this.peers.push(peer);
    peer.peers.push(this);
  }

  put(env: Envelope): number {
    if (!env.key || env.key.length === 0) throw new Error('key required');
    if (env.priority < 0 || env.priority > 255) throw new Error('priority out of range');
    if (env.seq < 0) throw new Error('seq must be >= 0');
    // store/query: keep last value per key (deterministic: highest seq wins)
    const prev = this.store.get(env.key);
    if (!prev || env.seq >= prev.seq) this.store.set(env.key, env);
    return this.deliver(env);
  }

  private deliver(env: Envelope): number {
    let count = 0;
    // local subscribers
    for (const [prefix, fns] of this.subs) {
      if (keyMatches(env.key, prefix)) {
        for (const fn of fns) { fn(env); count++; }
      }
    }
    // mesh peers (gossip) — they arbitrate on their own stores
    for (const peer of this.peers) {
      const pprev = peer.store.get(env.key);
      if (!pprev || wins(env, pprev)) peer.put(env);
    }
    return count;
  }

  subscribe(keyPrefix: string, fn: (e: Envelope) => void): () => void {
    const arr = this.subs.get(keyPrefix) ?? [];
    arr.push(fn);
    this.subs.set(keyPrefix, arr);
    return () => {
      const a = this.subs.get(keyPrefix);
      if (!a) return;
      const i = a.indexOf(fn);
      if (i >= 0) a.splice(i, 1);
    };
  }

  get(key: string): Envelope | undefined {
    return this.store.get(key);
  }
}

/** Build a small mesh of N nodes, fully connected (decentralized). */
export const createLocalMesh = (ids: string[]): LocalMesh[] => {
  const nodes = ids.map((id) => new LocalMesh(id));
  for (let i = 0; i < nodes.length; i++)
    for (let j = i + 1; j < nodes.length; j++) nodes[i].connect(nodes[j]);
  return nodes;
};
