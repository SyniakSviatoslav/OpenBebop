# Bebop — GitHub libraries & resources to STAR (operator list)

Generated 2026-07-12. This is the consolidated list of every open-source library
and project bebop uses, borrows ideas from, or integrates (Termux tools). Please
star them — open source survives on stars. 🌟

## 1. Rust crates bebop depends on (Cargo.toml)

- ratatui (TUI) — https://github.com/ratatui-org/ratatui
- crossterm (terminal) — https://github.com/crossterm-rs/crossterm
- clap (CLI args) — https://github.com/clap-rs/clap
- toml (config) — https://github.com/toml-rs/toml
- serde — https://github.com/serde-rs/serde
- serde_json — https://github.com/serde-rs/serde
- anyhow — https://github.com/dtolnay/anyhow
- thiserror — https://github.com/dtolnay/thiserror
- paste — https://github.com/dtolnay/paste
- chacha20poly1305 (AEAD) — https://github.com/RustCrypto/AEADs
- argon2 (KDF) — https://github.com/RustCrypto/password-hashes
- ml-kem (FIPS 203) — https://github.com/RustCrypto/KEMs
- ml-dsa (FIPS 204) — https://github.com/RustCrypto/signatures
- x25519-dalek — https://github.com/RustCrypto/Curves
- ed25519-dalek — https://github.com/RustCrypto/signatures
- getrandom — https://github.com/rust-random/getrandom
- signature — https://github.com/RustCrypto/signatures
- zeroize — https://github.com/RustCrypto/utils
- sha2 — https://github.com/RustCrypto/hashes
- hex — https://github.com/RustCrypto/utils
- iroh (P2P transport, proto-wire) — https://github.com/n0-computer/iroh
- zenoh (mesh, memory) — https://github.com/eclipse-zenoh/zenoh

## 2. Resources / projects ideas were borrowed from

- Hermes Agent (skill system, AGENTS.md, memory-first) — https://github.com/NousResearch/hermes-agent
- OpenCode (feed, agentic loop, TUI) — https://github.com/opencode-ai/opencode
- Claude Code (permission modes: plan/acceptEdits/bypass, headless -p) — https://github.com/anthropics/claude-code
- RustCrypto/signatures (ACVP KAT vectors, ML-DSA-65 byte-exact) — https://github.com/RustCrypto/signatures
- NIST FIPS 203 / 204 (PQ standards) — https://csrc.nist.gov/pubs/fips/203/final , https://csrc.nist.gov/pubs/fips/204/final
- Dota 2 (per-match scoreboard metaphor) — https://www.dota2.com
- XCOM 2 (after-action report / rewind metaphor) — https://www.firaxis.com/xcom-2

## 3. Native voice stack (offline, no AI in transcription)

- espeak-ng (TTS) — https://github.com/espeak-ng/espeak-ng
- piper (neural TTS, local) — https://github.com/rhasspy/piper
- whisper.cpp (STT, local) — https://github.com/ggml-org/whisper.cpp

## 4. Resource telemetry

- sysinfo (CPU/RAM/disk/OS) — https://github.com/GuillaumeGomez/sysinfo

## 5. Termux / recon / OSINT tools (dual-use — manual-enable, vuln-scanned)

- Cariddi (crawler) — https://github.com/edoardottt/cariddi
- ip-tracer — https://github.com/Kehlamar/ip-tracer
- chafa (image→terminal) — https://github.com/hpjansson/chafa
- neovim (editor) — https://github.com/neovim/neovim
- blackbird-osint — https://github.com/p1ngul1n0/blackbird
- webinfo — https://github.com/example-webinfo (verify; placeholder)
- onefetch (repo summary) — https://github.com/o2sh/onefetch
- aliens-eye — https://github.com/alien-eyes/aliens-eye
- dufs (file server) — https://github.com/sigoden/dufs
- lynx (browser) — https://github.com/ThomasDickey/lynx-snapshots
- nmap (scanner) — https://github.com/nmap/nmap
- masscan — https://github.com/robertdavidgraham/masscan
- rustscan — https://github.com/RustScan/RustScan
- naabu — https://github.com/projectdiscovery/naabu
- dnsx (dns-scanning) — https://github.com/projectdiscovery/dnsx
- spiderfoot — https://github.com/smicallef/spiderfoot
- termux-packages / termux-localhost — https://github.com/termux/termux-packages
- wormgpt — FLAGGED dual-use; NOT in default collection; opt-in only. (Verify repo before use.)

## 6. Operator's default collection (auto-enabled, changeable in settings)

The Rust crates in §1 + the borrowed-idea sources in §2 form the default
collection. Manage via `bebop coll` (planned). Every author above deserves a star.

---
Please star generously. Open source is a gift economy. 🌟
