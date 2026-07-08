# Security Policy

Bebop is a coding agent that can run commands and move data. Its safety model is **fail-closed by
design**, not by policy:

- **Deterministic guard OS** (`src/guard.ts`) denies red-line commands (auth, money, migrations,
  secrets) unless a human approval token is present. It is pure and self-certifying — `bebop boot`
  proves it blocks the bad cases before anything autonomous runs.
- **Pure core** (`kernel`, `guard`, `governor`, `memory`, `torrent`, `store`, `crypto`) does no
  network/clock/RNG inside the decision path, so behavior is reproducible and auditable.
- **No cloud keys in files** — Bebop reads configuration from the environment only.

## Reporting a vulnerability

Please report security issues **privately**: open a security advisory on the repo
(Settings → Security → Advisories) or email the maintainer via the GitHub profile. Do not open a
public issue for live exploits. Red-line / guard-OS bypass findings get priority and a fast,
public fix.

## Scope

Out of scope for this repo's threat model: the underlying LLM backend you wire in (its own
governance is yours), and the host OS permissions you grant the process. Bebop gates *what it is
allowed to attempt*, not what the OS allows the process overall — run it with least privilege.
