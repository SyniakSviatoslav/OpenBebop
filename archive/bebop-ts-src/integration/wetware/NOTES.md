# wetware (FinalSpark) — reverse-engineering notes

## Source (reverse-engineered)
- **FinalSpark Neuroplatform** (living neural organoids on multi-electrode arrays, cloud Python API,
  launched 2024-05-15). You send `Stimulus` (electrical impulses on a channel), read back `Spike`
  (action potentials). Non-deterministic, seconds/min latency, bio-substrate — a real "wetware
  compute" backend. Per operator directive 2026-07-08: red-line gates dropped; integrate ALL.

## Honest scope
- FinalSpark is NON-DETERMINISTIC (bio noise, latency). We integrate it behind a common
  `WetwareBackend` interface with a DETERMINISTIC LOCAL STUB (leaky-integrate-and-fire toy) so the node
  stays provable offline; the real bio-backend is an opt-in remote adapter. This is honesty, not a gate:
  a bio substrate cannot be a part of the kernel's deterministic money/verify path.

## Safety invariant captured
- **Bio-safe amplitude**: `BIO_MAX_MV = 50` — stimulus amplitude above this is REJECTED (tissue damage
  prevention). LIF threshold `20` mV. GREEN test uses amp `30`; RED uses `999`.

## Wiring (max-EV)
- Off the deterministic kernel path. Use as an OPTIONAL compute/exploration co-processor (e.g. spike-based
  anomaly detection on telemetry) behind a feature flag, never in the money/verify gate.

## Verified-by-Math
- `finalspark.test.ts`: GREEN deterministic LIF stub fires above threshold + returns spikes; RED rejects
  amplitude > BIO_MAX_MV / malformed stimulus.
