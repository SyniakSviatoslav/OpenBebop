// Native TypeScript equivalent of the RISC Zero guest `decide()` (guest/src/lib.rs).
//
// The algorithm is ported BYTE-FOR-BYTE from the Rust guest so a prover-less native
// run produces the exact same 32-byte digest as an in-circuit run would commit to
// the receipt journal.
//
// Determinism: same (state, cmd, ctx, counter) -> same 32-byte digest. Pure
// arithmetic only (wrapping add / rotate / multiply) — no time, no IO, no randomness.

export function rotr8(x: number, n: number): number {
  // 8-bit rotate right
  const v = x & 0xff;
  return ((v >>> n) | (v << (8 - n))) & 0xff;
}

export function rotl8(x: number, n: number): number {
  // 8-bit rotate left
  const v = x & 0xff;
  return ((v << n) | (v >>> (8 - n))) & 0xff;
}

export function hash32(data: Uint8Array, counter: number): Uint8Array {
  const h = new Uint8Array(32);
  for (let i = 0; i < 32; i++) {
    h[i] = (((counter & 0xff) as number)
      + ((i * 0x9e) & 0xff)
      + 0x24) & 0xff;
  }
  const FNV = 0x0100_0193; // 32-bit FNV prime
  for (let di = 0; di < data.length; di++) {
    const b = data[di];
    for (let i = 0; i < 32; i++) {
      const j = (i + 1) % 32;
      // acc = h[i] ^ b ^ (h[j] << 8)  -- 32-bit
      const acc = ((h[i] as number) ^ (b as number) ^ ((h[j] as number) << 8)) >>> 0;
      const mixed = (acc * FNV) >>> 0; // 32-bit wrapping multiply
      const rotated = ((mixed << 7) | (mixed >>> 25)) >>> 0; // rotr? no: rotate LEFT 7
      h[i] = rotated & 0xff;
    }
  }
  return h;
}

function u32le(n: number): Uint8Array {
  const o = new Uint8Array(4);
  o[0] = n & 0xff;
  o[1] = (n >>> 8) & 0xff;
  o[2] = (n >>> 16) & 0xff;
  o[3] = (n >>> 24) & 0xff;
  return o;
}

export function canonical(state: Uint8Array, cmd: Uint8Array, ctx: Uint8Array, counter: number): Uint8Array {
  const buf = new Uint8Array(16 + state.length + cmd.length + ctx.length);
  buf.set(u32le(counter), 0);
  buf.set(u32le(state.length), 4);
  buf.set(u32le(cmd.length), 8);
  buf.set(u32le(ctx.length), 12);
  buf.set(state, 16);
  buf.set(cmd, 16 + state.length);
  buf.set(ctx, 16 + state.length + cmd.length);
  return buf;
}

/** Deterministic decision. Returns the 32-byte digest. */
export function decide(state: Uint8Array, cmd: Uint8Array, ctx: Uint8Array, counter: number): Uint8Array {
  return hash32(canonical(state, cmd, ctx, counter), counter);
}

/**
 * Build the receipt journal exactly as the guest's main.rs commits it:
 *   digest(32) || counter(4) || stateLen(4) || cmdLen(4) || ctxLen(4) || state || cmd || ctx
 */
export function buildJournal(state: Uint8Array, cmd: Uint8Array, ctx: Uint8Array, counter: number): Uint8Array {
  const digest = decide(state, cmd, ctx, counter);
  const journal = new Uint8Array(32 + 16 + state.length + cmd.length + ctx.length);
  journal.set(digest, 0);
  journal.set(u32le(counter), 32);
  journal.set(u32le(state.length), 36);
  journal.set(u32le(cmd.length), 40);
  journal.set(u32le(ctx.length), 44);
  journal.set(state, 48);
  journal.set(cmd, 48 + state.length);
  journal.set(ctx, 48 + state.length + cmd.length);
  return journal;
}

export function toBytes(s: string): Uint8Array {
  return new TextEncoder().encode(s);
}
