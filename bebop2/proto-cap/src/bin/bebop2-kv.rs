//! bebop2-kv — CLI for the real hybrid (Ed25519 ⊕ ML-DSA-65) K/V signer.
//!
//! Commands:
//!   genkeys <master-hex>
//!       Prints two anchor lines: `<hex> role=K` then `<hex> role=V`.
//!       The hex encodes ed_pub (32 bytes) immediately followed by the
//!       1952-byte ML-DSA-65 public key.
//!
//!   sign <role=K|V> <master-hex> <msg-hex>
//!       Signs msg (hex) with the given role's full hybrid key and prints the
//!       hybrid signature as hex: ed_sig (64 bytes) ++ pq_sig (3309 bytes).
//!
//!   verify <anchor-line> <msg-hex> <sig-hex>
//!       Verifies the hybrid sig (RequireBoth) and prints `{"ok":bool}`.
//!
//! ZERO external dependencies. Hex I/O throughout (no base64 crate). The only
//! JSON emitted is the trivial `{"ok":true|false}` verify result, hand-rolled.

use std::process::exit;

use bebop_proto_cap::kv_signer::{
    derive_kv_keys, kv_pub, sign_hybrid, sig_from_hex, sig_to_hex, verify_hybrid, KvKey,
};

fn hex_decode(s: &str) -> Result<Vec<u8>, String> {
    let s = s.as_bytes();
    if s.len() % 2 != 0 {
        return Err("odd hex length".into());
    }
    let mut out = Vec::with_capacity(s.len() / 2);
    let mut i = 0;
    while i < s.len() {
        let hi = hex_val(s[i])?;
        let lo = hex_val(s[i + 1])?;
        out.push((hi << 4) | lo);
        i += 2;
    }
    Ok(out)
}

fn hex_val(c: u8) -> Result<u8, String> {
    match c {
        b'0'..=b'9' => Ok(c - b'0'),
        b'a'..=b'f' => Ok(c - b'a' + 10),
        b'A'..=b'F' => Ok(c - b'A' + 10),
        _ => Err(format!("invalid hex char: {}", c as char)),
    }
}

fn master_from_hex(arg: &str) -> Result<[u8; 32], String> {
    let bytes = hex_decode(arg)?;
    if bytes.len() != 32 {
        return Err(format!("master must be 32 bytes, got {}", bytes.len()));
    }
    let mut m = [0u8; 32];
    m.copy_from_slice(&bytes);
    Ok(m)
}

fn die(msg: &str) -> ! {
    eprintln!("error: {}", msg);
    exit(1);
}

/// Re-derive a role's full key from the master seed (no key persistence — the
/// master seed is the single trust root; the operator mints kv-genesis.txt
/// separately and never commits a seed here).
fn role_key(master: &[u8; 32], role: char) -> KvKey {
    let (k, v) = derive_kv_keys(master);
    match role {
        'K' => k,
        'V' => v,
        _ => die("role must be K or V"),
    }
}

fn cmd_genkeys(args: &[String]) {
    if args.len() != 1 {
        die("usage: genkeys <master-hex>");
    }
    let master = master_from_hex(&args[0]).unwrap_or_else(|e| die(&e));
    let (k, v) = derive_kv_keys(&master);
    println!("{}", kv_pub(&k).to_anchor_line('K'));
    println!("{}", kv_pub(&v).to_anchor_line('V'));
}

fn cmd_sign(args: &[String]) {
    if args.len() != 3 {
        die("usage: sign <role=K|V> <master-hex> <msg-hex>");
    }
    let role = args[0]
        .chars()
        .next()
        .unwrap_or('?');
    if role != 'K' && role != 'V' {
        die("role must be K or V");
    }
    let master = master_from_hex(&args[1]).unwrap_or_else(|e| die(&e));
    let msg = hex_decode(&args[2]).unwrap_or_else(|e| die(&e));
    let key = role_key(&master, role);
    let sig = sign_hybrid(&key, &msg);
    println!("{}", sig_to_hex(&sig));
}

fn cmd_verify(args: &[String]) {
    if args.len() != 3 {
        die("usage: verify <anchor-line> <msg-hex> <sig-hex>");
    }
    // anchor-line: "<hex> role=X"
    let (pub_anchor, _role) =
        bebop_proto_cap::kv_signer::KvPub::from_anchor_line(&args[0]).unwrap_or_else(|| die("bad anchor line"));
    let msg = hex_decode(&args[1]).unwrap_or_else(|e| die(&e));
    let sig = sig_from_hex(&args[2]).unwrap_or_else(|| die("bad sig hex"));
    let ok = verify_hybrid(&pub_anchor, &msg, &sig);
    println!("{{\"ok\":{}}}", ok);
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: bebop2-kv <genkeys|sign|verify> ...");
        exit(2);
    }
    match args[1].as_str() {
        "genkeys" => cmd_genkeys(&args[2..]),
        "sign" => cmd_sign(&args[2..]),
        "verify" => cmd_verify(&args[2..]),
        other => {
            eprintln!("unknown subcommand: {}", other);
            exit(2);
        }
    }
}
