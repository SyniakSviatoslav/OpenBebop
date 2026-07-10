# Tool Survey & Integration — 2026-07-10

> Reverse-engineered from a 150+ item operator dump (offsec recon, agent
> orchestration, eval/guardrails, math/control theory, plus a large tail of
> noise). Policy: **research → reverse-engineer → apply the CORE PATTERN
> natively (std-only, deterministic, falsifiable) → prune what's not needed.**
> Live external glue (APIs, model weights, binaries) stays OUTSIDE the
> deterministic core behind an eval gate — sovereign-core stays offline.

## Verdict buckets

- **INTEGRATE (done this pass):** OSINT naming enumeration (theHarvester/maigret/
  spiderfoot → `naming_osint`), field/L5 control-loop health (Kalman +
  limit-cycle → `field_kalman`/`limit_cycle_unstable`/`loop_health`).
- **DEFER (needs external service/model/UI, documented not blind-integrated):**
  headroom (token-compression proxy), supermemory (vector memory layer),
  markitdown (doc→md), webhackersweapons (tool index), DeepEval/garak (eval/red-
  team), LangGraph/Langflow (orchestration), Dify/n8n (agent UI), Crawl4ai/
  markitdown (ingestion), Shodan/Maltego/Spiderfoot *live* (need API keys),
  RAG/self-RAG, Temporal (durable exec), OpenCanary (decoy host), Pi-hole,
  HelixDB (graph DB — needs ground-truth eval before adoption).
- **NOISE / NOT NEEDED (pruned):** Nvidia/SkillSpector, Ideogram, music/TTS
  (Coqui/Artlist/VibeVoice/LuxTTS/KittenTTS), video (Remotion/ComfyUI/
  videouse), UI kits (shadcn/cult-ui/referor), payments (Priceghost/stripe),
  social/SEO/translation, crypto/NFT, generic "awesome lists", agent-chat
  front-ends.

## Integrated this pass (Verified-by-Math)

### 1. `naming_osint` — OSINT naming enumeration [research_patterns.rs]
- **Reverse-engineered from:** theHarvester, maigret, spiderfoot. Core pattern =
  enumerate a handle across N sources and *correlate* hits into one identity.
- **Native impl:** `naming_osint(handles, sources) -> HashMap<handle, Vec<source>>`.
  Deterministic, network-OFF. Fail-closed: empty input → empty map (never
  invents an identity).
- **Proof:** `mcp_harvest_correlates_handles` RED+GREEN (correlates 2 handles
  across 3 sources; empty → REFUSED). Live MCP sim: `HARVEST: 2 handles
  correlated: neo → github,gitlab`.

### 2. `field_kalman` + `limit_cycle_unstable` + `loop_health` — control-loop health [field.rs]
- **Reverse-engineered from:** the control-theory dossier (Kalman filter,
  limit cycles, Lyapunov/adaptive control). A control loop that orbits instead
  of settling is a *limit cycle*; a drifting estimate needs a *Kalman* filter.
- **Native impl:**
  - `field_kalman(z, q, r)` — scalar KF, gain `k = p/(p+r)`, deterministic.
  - `limit_cycle_unstable(s, min_flips, amp_band)` — bounded sign-flip detector.
  - `loop_health(s, q, r, drift, min_flips, amp_band) -> FieldVerdict` — fail-
    closed `Unhealthy` on oscillation OR drift; `Permit` only when stable+in-band.
- **Proof:** `loop_health_fails_closed_on_oscillation_and_drift` RED+GREEN
  (oscillation + drift → Unhealthy; stable → Permit; empty → Unhealthy). Live
  MCP sim: `LOOP_HEALTH: UNHEALTHY` on `[1,-1,1,-1,1,-1]`.

### MCP surface (now 12 tools)
`dispatch recall outfit scan plan audit field boundary wire sandbox recon
harvest loop_health`. New: `harvest` (OSINT naming), `loop_health` (control-loop
health). Both RED+GREEN tested via stdio sims.

## Deliberately NOT integrated (and why)

- **headroom / supermemory / markitdown** — valuable but need an external
  service/model or a network boundary that contradicts sovereign-core offline
  doctrine. Their *pattern* (token compression, durable memory, doc→md) is noted;
  glue lives behind an eval gate, not in the deterministic core.
- **Shodan/Maltego/Spiderfoot *live* recon** — require API keys + egress. The
  deterministic core models the *correlation logic* (`naming_osint`, `recon`);
  live source glue must be gated by `TargetScope` + eval before use.
- **LangGraph/Langflow/Dify/n8n** — orchestration/UIs; bebop already has a
  native `wire` 3-layer runtime (field ↔ living memory ↔ project). Re-importing
  a framework would bloat, not help.
- **Everything music/TTS/video/UI/payments/social** — orthogonal to bebop's
  deterministic security-agent mission; pruned.

## Test count
202 → 206 → 212 → 218 → **224** (208 bebop + 16 rust-core). 0 fail.
