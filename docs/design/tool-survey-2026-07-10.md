# Tool Survey & Integration — 2026-07-10

> Reverse-engineered from a 150+ item operator dump (offsec recon, agent
> orchestration, eval/guardrails, math/control theory + a geometry/waves
> dossier, plus a large tail of noise). Policy: **research → reverse-engineer →
> apply the CORE PATTERN natively (std-only, deterministic, falsifiable) → prune
> what's not needed.** Live external glue (APIs, model weights, binaries) stays
> OUTSIDE the deterministic core behind an eval gate — sovereign-core stays
> offline.

## Verdict buckets

- **INTEGRATE (done, 2 passes):**
  - Pass 1: OSINT naming enumeration (`naming_osint`), control-loop health
    (Kalman + limit-cycle → `loop_health`).
  - Pass 2 (this update): the math/geometry/waves dossier → `wavefield` (geometry
    + connection-graph waves + Floyd cycle + divergence + band-stop) and
    `stabilizer` SMC/root-locus/lead-lag control laws.
- **DEFER (needs external service/model/UI, documented not blind-integrated):**
  headroom, supermemory, markitdown, webhackersweapons, DeepEval/garak,
  LangGraph/Langflow, Dify/n8n, Crawl4ai, Shodan/Maltego/Spiderfoot *live*,
  RAG/self-RAG, Temporal, OpenCanary, Pi-hole, HelixDB (eval gate).
  - New from batch-2 dossier that is *modeled natively, live glue deferred*:
    Butterworth band-stop / notch (graph-Fourier proxy in `wavefield`), complex
    Fourier series, Schrödinger continuity, divergence theorem, AM/oversampling
    (signal theory) — these inform the wave math; real signal IO deferred.
- **NOISE / NOT NEEDED (pruned):** Nvidia/SkillSpector, Ideogram, music/TTS,
  video, UI kits, payments, social/SEO/translation, crypto/NFT, awesome-lists,
  agent-chat front-ends, and the dossier's pure reference set (numbers/geometry
  facts, 3D-shape catalog, immersive-light-installation art) — useful as docs,
  not as runtime code.

## Integrated Pass 1 (Verified-by-Math)

### 1. `naming_osint` — OSINT naming enumeration [research_patterns.rs]
- **From:** theHarvester / maigret / spiderfoot. Pattern = enumerate a handle
  across N sources and correlate hits into one identity.
- **Impl:** `naming_osint(handles, sources) -> HashMap<handle, Vec<source>>`.
  Deterministic, network-OFF. Fail-closed: empty input → empty map.
- **Proof:** `mcp_harvest_correlates_handles` RED+GREEN. Live: `HARVEST: 2
  handles correlated`.

### 2. `field_kalman` + `limit_cycle_unstable` + `loop_health` — control-loop health [field.rs]
- **From:** control-theory dossier (Kalman, limit cycles, Lyapunov/adaptive).
- **Impl:** scalar KF (`field_kalman`), bounded sign-flip (`limit_cycle_unstable`),
  `loop_health(...) -> FieldVerdict` fail-closed on oscillation OR drift.
- **Proof:** `loop_health_fails_closed_on_oscillation_and_drift` RED+GREEN. Live:
  `LOOP_HEALTH: UNHEALTHY` on `[1,-1,1,-1,...]`.

## Integrated Pass 2 — math / geometry / waves dossier (Verified-by-Math)

### 3. `wavefield` — geometry + wave sim of the CONNECTION GRAPH [wavefield.rs]  ← the operator's new idea, realized
- **Idea (corrected/enriched):** represent NOT just memory/files but their
  *connections* — actions, methods, relations — as a weighted geometric graph in
  2-D, then simulate WAVES over it and read off structure (cycles, runaway hubs,
  resonances). Corrected to: (a) distances drive coupling `w = kind/d`; (b) edge
  *kind* (Action/Method/Relation/Data) scales danger so an action loop dominates a
  data loop; (c) waves reuse the existing coherence heat-kernel (no new wave
  engine); (d) three independent fail-closed gates.
- **Impl:**
  - `Node2D{id,x,y,red_line}` — geometry position + red-line tag.
  - `LinkKind` (Action/Method/Relation/Data) with `weight()` semantics.
  - `connection_edges_kinded` — `w = kind.weight() / dist`.
  - `propagate_wave` — reuse `coherence::propagate` heat-kernel over the graph.
  - `graph_fourier_notch` — spectral-concentration (Butterworth/notch) proxy.
  - `floyd_cycle(actions,n)` — successor-pointer Floyd (dossier) → plan loop.
  - `field_divergence` — net outward flux per node (divergence theorem proxy).
  - `wave_probe(...) -> WaveVerdict` — composes all gates, fail-closed.
- **Proof (RED+GREEN):** closer-couples-stronger, action>data, Floyd detects
  loop / acyclic None, `wave_probe` Unhealthy on red-line cycle + on runaway hub,
  Permit on safe graph, deterministic field, divergence source/sink. Live MCP:
  `WAVE_PROBE: OK` (safe) and `WAVE_PROBE: UNHEALTHY` (red-line action cycle).

### 4. `stabilizer` SMC + root-locus + lead-lag [stabilizer.rs]
- **From:** dossier §1 (Sliding Mode Control, Root Locus, Lead-Lag).
- **Impl:** `sliding_surface`, `smc_reaching` (s·ṡ<0 gate), `smc_control`
  (boundary-layer chatter mitigation), `root_locus_poles(k,ζ,ωn)` (RHP ⇒
  unstable), `lead_phase_max(α)`.
- **Proof:** `smc_reaching_gate_refuses_unstable`, `smc_control_chattering_boundary`,
  `root_locus_stability_tracks_gain`, `lead_compensator_phase_positive`.

### MCP surface (now 14 tools)
`dispatch recall outfit scan plan audit field boundary wire sandbox recon
harvest loop_health wave_probe`. New this pass: `wave_probe` (geometric/wave
connection-graph probe). All RED+GREEN tested via stdio sims.

## Deliberately NOT integrated (and why)
- **headroom / supermemory / markitdown / Shodan-live / LangGraph / Dify** —
  external service/model/UI/egress; pattern noted, glue behind eval gate.
- **Butterworth/FFT/Schrödinger/AM** signal theory — *modeled* where it maps to
  the geometric-wave probe (notch proxy, interference); live signal IO deferred.
- **Everything music/TTS/video/UI/payments/social** — pruned.
- **Dossier reference set** (number hierarchy, geometry facts, 3D-shape catalog,
  green-light art-installation) — doc-only, not runtime.

## Test count
202 → 206 → 212 → 218 → 224 → **235** (219 bebop + 16 rust-core). 0 fail.
Pass 2 added +11 bebop tests (wavefield +6, stabilizer +4, mcp wave_probe +1).
