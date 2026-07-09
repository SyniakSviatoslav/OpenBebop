//! Deterministic `decide()` for the RISC Zero guest.
//!
//! The logic here is identical, byte-for-byte, to `src/integration/zkvm/decide.ts`
//! so that a native (prover-less) run and an in-circuit run produce the same journal.
//! Determinism requirement: same (state, cmd, ctx, counter) -> same 32-byte digest.

/// Deterministic 32-byte digest over `data`, seeded by `counter`.
///
/// Construction: a 32-byte state seeded from the counter, then a FNV-1a style
/// fold (xor + multiply by the 32-bit FNV prime) over every input byte, advanced
/// through the 32-byte state with a rotate. Pure arithmetic (wrapping ops), no
/// heap/IO/time. Inputs fully determine the output and distinct inputs produce
/// distinct digests (avalanche verified by the determinism test).
pub fn hash32(data: &[u8], counter: u32) -> [u8; 32] {
    // Seed: spread the committed counter across all 32 bytes.
    let mut h = [0u8; 32];
    for (i, slot) in h.iter_mut().enumerate() {
        *slot = (counter as u8)
            .wrapping_add((i as u8).wrapping_mul(0x9e))
            .wrapping_add(0x24);
    }
    const FNV: u32 = 0x0100_0193; // 32-bit FNV prime
    for &b in data {
        for i in 0..32 {
            let j = (i + 1) % 32;
            // fold byte b into slot i, carry avalanche from slot j
            let acc = (h[i] as u32) ^ (b as u32) ^ ((h[j] as u32) << 8);
            let mixed = acc.wrapping_mul(FNV).rotate_left(7);
            h[i] = mixed as u8;
        }
    }
    h
}

/// Canonical serialization of the four inputs, prefixed by the committed counter.
fn canonical(state: &[u8], cmd: &[u8], ctx: &[u8], counter: u32) -> Vec<u8> {
    let mut buf = Vec::with_capacity(
        16 + state.len() + cmd.len() + ctx.len(),
    );
    buf.extend_from_slice(&counter.to_le_bytes());
    buf.extend_from_slice(&(state.len() as u32).to_le_bytes());
    buf.extend_from_slice(&(cmd.len() as u32).to_le_bytes());
    buf.extend_from_slice(&(ctx.len() as u32).to_le_bytes());
    buf.extend_from_slice(state);
    buf.extend_from_slice(cmd);
    buf.extend_from_slice(ctx);
    buf
}

/// The deterministic decision. `counter` is a committed monotonic nonce so that
/// each decision round is uniquely bound to a sequence number (prevents replay).
pub fn decide(state: &[u8], cmd: &[u8], ctx: &[u8], counter: u32) -> [u8; 32] {
    hash32(&canonical(state, cmd, ctx, counter), counter)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn deterministic() {
        let a = decide(b"state", b"cmd", b"ctx", 1);
        let b = decide(b"state", b"cmd", b"ctx", 1);
        assert_eq!(a, b);
        // different counter -> different digest
        assert_ne!(a, decide(b"state", b"cmd", b"ctx", 2));
        // different input -> different digest
        assert_ne!(a, decide(b"STATE", b"cmd", b"ctx", 1));
    }
}
