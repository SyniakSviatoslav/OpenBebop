//! RISC Zero guest entrypoint.
//!
//! Reads the encoded input from the zkVM stdin, computes `decide()`, and commits
//! the authenticated JOURNAL (digest || counter || lengths || inputs) so a
//! verifier can later check a receipt against claimed inputs.
//!
//! The risc0 `env` API only exists inside the actual zkVM (`--target
//! riscv32im-risc0-zkvm-elf`). On the HOST target (plain `cargo test`) we use a
//! stub `main` so the crate still compiles and its `#[cfg(test)]` determinism
//! checks run. This keeps the determinism proof runnable without the prover.

#[cfg(target_os = "zkvm")]
mod zkvm_main {
    use std::io::Read;
    use risc0_zkvm::guest::env;
    use zkvm_guest::decide;

    fn read_u32(buf: &[u8], off: usize) -> u32 {
        u32::from_le_bytes([buf[off], buf[off + 1], buf[off + 2], buf[off + 3]])
    }

    pub fn main() {
        // Input layout: counter(4) || stateLen(4) || cmdLen(4) || ctxLen(4) || state || cmd || ctx
        let mut input = Vec::new();
        env::stdin().read_to_end(&mut input).unwrap();

        let counter = read_u32(&input, 0);
        let state_len = read_u32(&input, 4) as usize;
        let cmd_len = read_u32(&input, 8) as usize;
        let ctx_len = read_u32(&input, 12) as usize;
        let mut p = 16;
        let state = &input[p..p + state_len];
        p += state_len;
        let cmd = &input[p..p + cmd_len];
        p += cmd_len;
        let ctx = &input[p..p + ctx_len];

        let digest = decide(state, cmd, ctx, counter);

        // Journal: digest(32) || counter(4) || stateLen(4) || cmdLen(4) || ctxLen(4) || state || cmd || ctx
        let mut journal = Vec::with_capacity(32 + 16 + state_len + cmd_len + ctx_len);
        journal.extend_from_slice(&digest);
        journal.extend_from_slice(&counter.to_le_bytes());
        journal.extend_from_slice(&(state_len as u32).to_le_bytes());
        journal.extend_from_slice(&(cmd_len as u32).to_le_bytes());
        journal.extend_from_slice(&(ctx_len as u32).to_le_bytes());
        journal.extend_from_slice(state);
        journal.extend_from_slice(cmd);
        journal.extend_from_slice(ctx);

        env::commit_slice(&journal);
    }
}

#[cfg(target_os = "zkvm")]
fn main() {
    zkvm_main::main();
}

/// Host-target stub: run a sample decision so `cargo test` (which compiles the
/// bin) succeeds and the determinism unit tests in lib.rs execute. No risc0 dep.
#[cfg(not(target_os = "zkvm"))]
fn main() {
    let digest = zkvm_guest::decide(b"state-0001", b"allow", b"epoch-7", 1);
    println!("host decide sample digest: {:?}", &digest[..4]);
}
