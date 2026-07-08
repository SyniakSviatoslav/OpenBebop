# Backends & routing

Bebop is **backend-agnostic**. A **Task Router** classifies each task and routes to the
**cheapest adequate backend** — local models, cloud APIs, or a native doer.

## The Backend interface

Implement `Backend` in `src/backend.ts`:

```ts
interface Backend {
  name: string;
  run(task: Task): { ok: boolean; summary: string; exitCode: number };
}
```

`runBackend(backend, task)` executes it and feeds the result into the unified token ledger
(`token.ts`) — no backend meters its own tokens.

## Routing: cheapest adequate

`src/router.ts` classifies a task into a class (`read` / `write` / `reason` / `creativity` /
`exec` / `doer` / `redline`) and returns the cheapest model that satisfies it:

```ts
route('reason')  // -> { model: 'opus', rationale: '...' }
enforceRouting('reason', model)  // { ok, note }
```

`src/routing.ts` (`probeAll` / `selectBackend`) probes which backends are installed/available
and selects among them. The CLI shows this with `bebop status` and `bebop route <class>`.

## Bring your own model

1. Add a `Backend` implementation (local Ollama, an OpenAI-compatible API, a native doer, …).
2. Register it in `probeAll` / `selectBackend`.
3. The router picks it by capability + cost. No core change needed.

## Why this matters

- **Cost-aware** — cheap tasks never burn an expensive model.
- **Portable** — point Bebop at whatever you have; the core doesn't care.
- **Falsifiable** — `router.test.ts` asserts the routing decision for each class, RED+GREEN.

## Token ledger

All backends report usage into one ledger (`token.ts`) so you get a single, comparable cost view
across models and providers — no per-backend accounting drift.
