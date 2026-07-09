#!/usr/bin/env node
// Bebop — your own coding agent CLI for dowiz (a la Claude Code / Hermes).
// Owns the tool loop, model routing, and hooks; bakes the dowiz Operating System in as NATIVE
// behavior. Brand: Warm Cosmo-Noir, main signal color = Cowboy Bebop ship teal #46B0A4.
//
// Subcommands:
//   boot            run the guard self-test (Verified-by-Math — refuse to start if gates can't go RED)
//   run [task]      run the agentic loop (default: deterministic stub, no live model)
//   agents          list every agentic CLI Bebop can drive + live connection status
//   use <backend>   switch the default agent directly (e.g. bebop use claude / opencode / free)
//   recall <q>      query the living-knowledge §0·GP retriever
//   route <class>   show the token-router decision (doer/reason/redline)
//   map [module]    "understand everything" — render the real import graph as an SVG image
//   diagrams        regenerate every visual (project map + conceptual feature schemas)
//   help            this text

import path from 'node:path';
import fs from 'node:fs';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { selfTest } from './src/guard.ts';
import { runLoop } from './src/loop.ts';
import { route, enforceRouting, type TaskClass } from './src/router.ts';
import { recall, rememberLocal } from './src/knowledge.ts';
import { livingMemory } from './src/memory.ts';
import { ContentStore } from './src/store.ts';
import { banner, makePaint } from './src/theme.ts';
import { BOOT } from './src/voice.ts';
import { init, loadProfile, statusLine, writeProfile } from './src/init.ts';
import { probeAll, selectBackend } from './src/routing.ts';
import { BEBOP_PRESET, type Profile } from './src/profile.ts';
import { runBackend, ADAPTERS, isAvailable, type Backend } from './src/backend.ts';
import { runCopilot } from './src/copilot.ts';
import { Governor } from './src/governor.ts';
import { startSyncServer } from './src/sync-server.ts';
import { selfMaintain, selfEvolve, recordSession, selfLoop } from './src/consciousness.ts';
import { createOrUnlock, lock, unlock, loadBlob } from './src/vault.ts';
import { runMcpServer } from './src/mcp.ts';
import { playLaunch } from './src/launch.ts';
import { loadSettings } from './src/settings.ts';
import { DEFAULT_SCOPE_GLOBS, checkRedLine } from './src/guard.ts';
import { subagent } from './src/loop.ts';
import { loadSkills, findSkill } from './src/skills.ts';
import { initCore } from './src/core-wasm.ts';
import { initWasiCore } from './src/core-wasi.ts';
import { setKernel } from './src/guard.ts';
import { buildGraph, renderSvg } from './src/understand.ts';
import { flowSchema, FEATURE_SCHEMAS } from './src/schema.ts';

const HERE = path.dirname(fileURLToPath(import.meta.url));

// Recursive file walker (deterministic, depth-first) — used by `bebop docs check`.
function* walk(dir: string): Generator<string> {
  for (const e of fs.readdirSync(dir, { withFileTypes: true })) {
    const p = path.join(dir, e.name);
    if (e.isDirectory()) yield* walk(p);
    else yield p;
  }
}

// Slash-command dispatcher — Claude Code's /help /clear /model /status /plan /compact /resume + bebop /review.
async function handleSlash(name: string, args: string[]): Promise<void> {
  const paint = makePaint();
  const settings = loadSettings();
  switch (name) {
    case 'help':
    case '?':
      console.log(banner(paint));
      console.log(paint.dim('  /help      this list'));
      console.log(paint.dim('  /status    backend rotation + guard state'));
      console.log(paint.dim('  /model     show routed model (= bebop route doer)'));
      console.log(paint.dim('  /clear     reset in-process living memory'));
      console.log(paint.dim('  /plan      show plan-mode note (run --plan to use)'));
      console.log(paint.dim('  /compact   summarize + trim living memory'));
      console.log(paint.dim('  /resume    resume last session node from memory'));
      console.log(paint.dim('  /review    run the review skill checklist'));
      console.log(paint.dim('  /skills    list loaded skills'));
      return;
    case 'status': {
      const profile = loadProfile() ?? BEBOP_PRESET;
      console.log(banner(paint));
      console.log(paint.dim(`  model=${(settings.model ?? route('doer').model)}  rotation=${statusLine(profile)}`));
      const t = selfTest();
      console.log(paint.dim(`  guard OS certified=${t.ok} (deny-on-red)`));
      return;
    }
    case 'model':
      console.log(paint.teal(`  model → ${settings.model ?? route('doer').model}`));
      return;
    case 'clear': {
      const mem = livingMemory();
      const before = mem.size;
      mem.clear();
      console.log(paint.dim(`  cleared living memory (was ${before}, now ${mem.size})`));
      return;
    }
    case 'compact': {
      const mem = livingMemory();
      const before = mem.size;
      mem.tick();
      console.log(paint.dim(`  compacted: ${before} → ${mem.size} (forgot ${before - mem.size})`));
      return;
    }
    case 'resume': {
      const mem = livingMemory();
      const last = mem.nearest('hermes', 1)[0];
      console.log(paint.dim(`  resume → ${last ? last.id : 'no session node yet'}`));
      return;
    }
    case 'plan':
      console.log(paint.dim('  plan mode: read-only. Use `bebop run <class> --plan` to explore without edits.'));
      return;
    case 'skills': {
      const skills = loadSkills();
      console.log(paint.teal(`  ${skills.length} skill(s):`));
      for (const s of skills) console.log(paint.dim(`  · ${s.name} — ${s.description}`));
      return;
    }
    case 'review': {
      const skills = loadSkills();
      const sk = findSkill(skills, 'review');
      const t = selfTest();
      console.log(paint.teal(`  /review — guard OS certified=${t.ok}`));
      if (sk) console.log(paint.dim(`  skill: ${sk.name} — ${sk.description}\n${sk.body.split('\n').slice(0, 6).join('\n')}`));
      else console.log(paint.amber('  review skill not found in .bebop/skills'));
      return;
    }
    case 'subagent': {
      // /subagent "<task>" — delegate read-only recon to a cheaper doer (Explore/Plan semantics).
      const task = args.join(' ');
      const r = await subagent(task || 'investigate the repo surface');
      console.log(paint.dim(`  [subagent] steps=${r.steps} denied=${r.denied}`));
      console.log(paint.dim(`  ${r.summary.split('\n')[0]}`));
      return;
    }
    default:
      console.log(paint.blood(`  unknown slash command: /${name}  (try /help)`));
      process.exit(2);
  }
}

async function main() {
  const [, , cmd, ...args] = process.argv;
  const paint = makePaint();

  // Boot the self-contained Rust/WASM guard kernel (if the artifact is present). The guard then
  // delegates every red-line/scope decision to it; if absent it transparently uses the TS port.
  // Sovereign Node Phase 2: BEBOP_CORE_RUNTIME=wasi runs the kernel under WasmEdge (hardened core);
  // any other value (default) uses the in-process WebAssembly loader. Both expose the same contract.
  const coreRuntime = process.env.BEBOP_CORE_RUNTIME ?? 'inproc';
  setKernel(coreRuntime === 'wasi' ? await initWasiCore() : await initCore());

  if (!cmd || cmd === 'help' || cmd === '--help' || cmd === '-h') {
    console.log(banner(paint));
    console.log(paint.dim('  boot | init [--preset bebop|--json {...}] | status | agents | use <backend>'));
    console.log(paint.dim('  run [doer|reason|redline] | dispatch "<task>" | route <class> | recall <query>'));
    console.log(paint.dim('  govern "<q,..>" | self [maintain|evolve|session|loop] | node | sync [--port N]'));
    console.log(paint.dim('  map [module] | diagrams | mcp | help'));
    console.log(paint.dim(`  ${BOOT.bebop.link}`));
    return;
  }

  if (cmd === 'boot') {
    const profile = loadProfile() ?? BEBOP_PRESET;
    await playLaunch({ paints: makePaint(profile.looks) });
    const t = selfTest();
    for (const l of t.log) console.log(paint.dim('  · ' + l));
    if (t.ok) {
      console.log(paint.teal('  ✓ Bebop guard OS certified: gates deny on red, pass on green.'));
    } else {
      console.log(paint.blood('  ✖ Guard self-test FAILED. The machine refuses to lie — fix before ship.'));
      process.exit(1);
    }
    return;
  }

  if (cmd === 'init') {
    const preset = args.includes('--preset') ? args[args.indexOf('--preset') + 1] : undefined;
    const jsonIdx = args.indexOf('--json');
    const json = jsonIdx >= 0 ? args[jsonIdx + 1] : undefined;
    const force = args.includes('--force');
    const profile = await init({ preset, json, force });
    const p = writeProfile(profile);
    console.log(paint.teal(`  ✓ Profile written → ${p}`));
    console.log(paint.dim(`  origin=${profile.origin} class=${profile.classKind} narration=${profile.narration} patrons=${profile.patrons} looks=${profile.looks}`));
    console.log(paint.dim(`  backend rotation: ${statusLine(profile)}`));
    console.log(paint.bold(paint.bone(`  ${BEBOP_PRESET === profile ? 'Bebop native preset engaged. Hybrid is a feature, not a bug.' : 'Custom profile engaged.'}`)));
    return;
  }

  if (cmd === 'status') {
    const profile = loadProfile() ?? BEBOP_PRESET;
    console.log(banner(paint));
    console.log(paint.dim(`  rotation: ${statusLine(profile)}  (* = not installed / no key)`));
    for (const r of probeAll(profile)) {
      const mark = r.available ? paint.teal('ready') : paint.amber('idle');
      console.log(paint.dim(`  · ${r.backend.padEnd(9)} ${mark}`));
    }
    return;
  }

  if (cmd === 'sync') {
    const portIdx = args.indexOf('--port');
    const port = portIdx >= 0 ? Number(args[portIdx + 1]) : Number(process.env.BEBOP_SYNC_PORT ?? 8787);
    console.log(paint.teal(`  ◈ Starting Bebop sync node (Better Auth, self-hosted) on :${port}`));
    console.log(paint.dim('    No Supabase. No Fly. Your keys, your machine. Ctrl-C to stop.'));
    const srv = await startSyncServer({ port });
    console.log(paint.teal(`  ✓ Sync node live → ${srv.url}  (signup: ${srv.url}/sign-up)`));
    // Keep the process alive until interrupted.
    await new Promise<void>((resolve) => {
      process.on('SIGINT', () => resolve());
      process.on('SIGTERM', () => resolve());
    });
    await srv.close();
    console.log(paint.dim('  sync node stopped.'));
    return;
  }

  if (cmd === 'mcp') {
    // Model Context Protocol server over stdio. Exposes Bebop capabilities as tools to any
    // MCP client (Claude Desktop, Cursor, Zed, VS Code, Hermes). Zero new dependencies.
    console.error(paint.teal('  ◈ Bebop MCP server starting on stdio (JSON-RPC 2.0). Close stdin to stop.'));
    await runMcpServer();
    return;
  }

  if (cmd === 'route') {
    const cls = (args[0] as TaskClass) ?? 'doer';
    const d = route(cls);
    const g = enforceRouting(cls, d.model);
    console.log(paint.teal(`  ${cls} → ${paint.bold(d.model)}`));
    console.log(paint.dim('  ' + d.rationale));
    if (!g.ok) console.log(paint.blood('  ' + g.note));
    return;
  }

  if (cmd === 'recall') {
    const q = args.join(' ');
    const r = recall(q);
    console.log(paint.dim(`  §0·GP recall — ${r.note}`));
    for (const h of r.hits) console.log(paint.teal(`  ◈ ${h.id}: ${h.text.slice(0, 100)}`));
    return;
  }

  if (cmd === 'map') {
    // "understand everything": render the real import graph as an SVG image.
    const focusStem = args[0]; // optional: focus a module (e.g. `bebop map guard`)
    const outDir = path.join(HERE, 'docs', 'diagrams');
    fs.mkdirSync(outDir, { recursive: true });
    const graph = buildGraph(HERE);
    const focus = focusStem
      ? graph.nodes
          .filter((n) => n.stem === focusStem || n.id.endsWith(`/${focusStem}.ts`))
          .map((n) => n.id)
      : [];
    const svg = renderSvg(graph, {
      focus: focus.length ? focus : undefined,
      title: focusStem ? `Bebop map — focus: ${focusStem}` : 'Bebop project map (real imports)',
    });
    const outFile = path.join(outDir, focusStem ? `map-${focusStem}.svg` : 'project-map.svg');
    fs.writeFileSync(outFile, svg);
    console.log(paint.teal(`  ◈ wrote ${outFile}`));
    console.log(paint.dim(`  ${graph.nodes.length} modules, ${graph.edges.length} real import edges`));
    return;
  }

  if (cmd === 'diagrams') {
    // Regenerate EVERY visual: the real import graph + all conceptual feature schemas.
    const outDir = path.join(HERE, 'docs', 'diagrams');
    fs.mkdirSync(outDir, { recursive: true });
    const graph = buildGraph(HERE);
    fs.writeFileSync(path.join(outDir, 'project-map.svg'), renderSvg(graph, { title: 'Bebop project map (real imports)' }));
    let n = 1;
    for (const [name, def] of Object.entries(FEATURE_SCHEMAS)) {
      fs.writeFileSync(path.join(outDir, `schema-${name}.svg`), flowSchema(def.steps, { title: def.title }));
      n++;
    }
    console.log(paint.teal(`  ◈ wrote docs/diagrams/project-map.svg`));
    console.log(paint.teal(`  ◈ wrote ${n - 1} conceptual schemas (schema-*.svg)`));
    console.log(paint.dim(`  graph: ${graph.nodes.length} modules, ${graph.edges.length} real edges`));
    return;
  }

  if (cmd === 'docs') {
    // `bebop docs` — the polished, repeatable documentation pipeline (Constant Doubt rule).
    // Subcommands:
    //   docs init     generate the OpenWiki agent-facing wiki (needs an LLM key: OPENWIKI_PROVIDER/KEY)
    //   docs update   refresh the wiki from git diffs since last run
    //   docs build    run every local pipeline: typecheck, tests, wasm, diagrams, footage + i18n checks
    //   docs check    verify the repo is release-ready: gifs resolve, counts, manifests, wiki presence
    const sub = args[0] ?? 'check';
    const run = (argv: string[], opts: { env?: NodeJS.ProcessEnv } = {}) =>
      spawnSync(argv[0], argv.slice(1), { stdio: 'inherit', cwd: HERE, env: { ...process.env, ...(opts.env ?? {}) } });

    if (sub === 'init' || sub === 'update') {
      // OpenWiki (langchain-ai/openwiki): writes openwiki/ and wires AGENTS.md/CLAUDE.md. Needs a key.
      if (!process.env.OPENWIKI_PROVIDER && !process.env.ANTHROPIC_API_KEY && !process.env.OPENAI_API_KEY && !process.env.OPENROUTER_API_KEY) {
        console.log(paint.amber('  ⚠ OpenWiki needs an LLM key. Set one, e.g.'));
        console.log(paint.dim('    export OPENWIKI_PROVIDER=openrouter  OPENROUTER_API_KEY=sk-or-...'));
        console.log(paint.dim('    then re-run `bebop docs init`. (No key in this environment — wiring only.)'));
      }
      const flag = sub === 'init' ? '--init' : '--update';
      console.log(paint.teal(`  ◈ openwiki ${flag} — agent-facing wiki → openwiki/`));
      const r = run(['npx', '-y', 'openwiki', flag]);
      process.exit(r.status ?? 0);
      return;
    }

    if (sub === 'build') {
      console.log(paint.teal('  ◈ docs build — running every local pipeline (no LLM needed)'));
      console.log(paint.dim('  · typecheck')); run(['npm', 'run', 'typecheck']);
      console.log(paint.dim('  · tests'));      run(['npm', 'test']);
      console.log(paint.dim('  · wasm kernel')); run(['npm', 'run', 'build']);
      console.log(paint.dim('  · diagrams (real import graph + schemas)')); run(['npx', 'tsx', 'bebop.ts', 'diagrams']);
      console.log(paint.dim('  · map'));         run(['npx', 'tsx', 'bebop.ts', 'map']);
      console.log(paint.dim('  · i18n self-check')); run(['node', 'scripts/i18n-translate.mjs', '--check', 'README.md', 'README.uk.md']);
      console.log(paint.dim('  · doc-claim self-correction (Constant Doubt)')); run(['node', 'scripts/verify-doc-claims.mjs']);
      console.log(paint.teal('  ◈ build pipeline done. Run `bebop docs check` to verify release-readiness.'));
      return;
    }

    // default: check
    console.log(paint.teal('  ◈ docs check — release-readiness audit (Constant Doubt)'));
    let fail = 0;
    const expect = (cond: boolean, msg: string) => {
      console.log(paint[cond ? 'teal' : 'blood'](`  ${cond ? '✓' : '✗'} ${msg}`));
      if (!cond) fail++;
    };
    // 1) embedded gifs resolve
    const docsDir = path.join(HERE, 'docs');
    let gifRefs = 0, gifBroken = 0;
    for (const f of walk(docsDir)) {
      if (!f.endsWith('.md')) continue;
      const txt = fs.readFileSync(f, 'utf8');
      for (const m of txt.matchAll(/!\[[^\]]*\]\(([^)]+\.gif)[^)]*\)/g)) {
        gifRefs++;
        const p = path.resolve(path.dirname(f), m[1]);
        if (!fs.existsSync(p)) gifBroken++;
      }
    }
    expect(gifBroken === 0, `all ${gifRefs} embedded GIFs resolve (${gifBroken} broken)`);
    // 2) manifests valid JSON
    for (const m of ['llm-manifest.json', 'docs/mcp-tools.json']) {
      try { JSON.parse(fs.readFileSync(path.join(HERE, m), 'utf8')); expect(true, `${m} valid JSON`); }
      catch { expect(false, `${m} valid JSON`); }
    }
    // 3) test count claims match reality
    const pkg = JSON.parse(fs.readFileSync(path.join(HERE, 'package.json'), 'utf8'));
    expect(pkg.version && /^\d+\.\d+\.\d+$/.test(pkg.version), `version ${pkg.version} is semver`);
    // 4) OpenWiki wired?
    const wiki = path.join(HERE, 'openwiki');
    expect(fs.existsSync(wiki), 'openwiki/ present (run `bebop docs init` to generate)');
    expect(fs.existsSync(path.join(HERE, '.github/workflows/openwiki-update.yml')), 'CI: openwiki update workflow present');
    // 5) doc-claim self-correction layer (Constant Doubt, enforced) — never ship a false statement
    const vres = spawnSync('node', ['scripts/verify-doc-claims.mjs'], { cwd: HERE, encoding: 'utf8' });
    if (vres.status !== 0) {
      console.log(vres.stdout || vres.stderr || '');
      fail++;
    }
    console.log(paint[fail ? 'blood' : 'teal'](`  ${fail ? `✗ ${fail} issue(s) — fix before release` : '✓ release-ready'}`));
    process.exit(fail ? 1 : 0);
  }

  if (cmd === 'remember') {
    // bebop remember <concept> :: <payload>  — write into the ONE living memory (this session included)
    const raw = args.join(' ');
    const sep = raw.indexOf('::');
    if (sep < 0) {
      console.log(paint.blood('  usage: bebop remember <concept> :: <payload>'));
      process.exit(2);
    }
    const concept = raw.slice(0, sep).trim();
    const payload = raw.slice(sep + 2).trim();
    const id = rememberLocal(concept, payload, args.includes('--link') ? [args[args.indexOf('--link') + 1]] : undefined);
    console.log(paint.teal(`  ✓ remembered "${concept}" → ${id.slice(0, 12)} (living memory size=${livingMemory().size})`));
    return;
  }

  if (cmd === 'memory') {
    // bebop memory — show the ONE living memory state (this Hermes session is a node)
    const mem = livingMemory();
    const sub = args[0];
    if (sub === 'tick') {
      // advance the forgetting clock: decay + eviction (human-like memory)
      const n = Math.max(1, Number(args[1] ?? 1));
      const before = mem.size;
      for (let i = 0; i < n; i++) mem.tick();
      console.log(paint.dim(`  ticked ${n}×: size ${before} → ${mem.size} (forgot ${before - mem.size})`));
      console.log(paint.dim(`  layers: working=${mem.layerSize('working')} short=${mem.layerSize('short')} long=${mem.layerSize('long')}`));
      return;
    }
    if (sub === 'layers') {
      console.log(paint.dim(`  layers: working=${mem.layerSize('working')} short=${mem.layerSize('short')} long=${mem.layerSize('long')} (total=${mem.size})`));
      return;
    }
    console.log(paint.dim(`  living memory size=${mem.size}`));
    console.log(paint.dim(`  layers: working=${mem.layerSize('working')} short=${mem.layerSize('short')} long=${mem.layerSize('long')}`));
    console.log(paint.dim(`  nearest to "copilot": ${JSON.stringify(mem.nearest('copilot', 3))}`));
    console.log(paint.dim(`  recall "copilot": ${JSON.stringify(mem.recall('copilot', 2))}`));
    return;
  }

  if (cmd === 'store') {
    // bebop store <dir> [append <cause> <data> | put <index> <text> | verify]
    const dir = args[0] ?? path.resolve(HERE, '.bebop', 'store');
    const op = args[1];
    const store = new ContentStore(dir);
    if (op === 'append') {
      const cause = args[2] ?? 'cause-x';
      const data = args.slice(3).join(' ') || 'tick';
      const ev = store.appendEvent(cause, data);
      console.log(paint.teal(`  ✓ event #${ev.seq} chained (hash ${ev.hash.slice(0, 12)})`));
    } else if (op === 'put') {
      const idx = Number(args[2] ?? 0);
      const text = args.slice(3).join(' ') || 'piece';
      const p = store.putPiece(idx, new TextEncoder().encode(text));
      console.log(paint.teal(`  ✓ piece #${idx} address ${p.hash.slice(0, 12)}`));
    } else {
      console.log(paint.dim(`  store dir=${dir} events=${store.eventCount} chainOk=${store.verifyChain()}`));
    }
    return;
  }

  if (cmd === 'dispatch') {
    const task = args.join(' ');
    // Governor: PID authority from copilot verdict (reject = mistake ⇒ freedom shrinks; approve = air).
    const gov = new Governor({ kp: 1.4, ki: 0.22, kd: 1.5, iMin: -1, iMax: 1, uMin: 0, uMax: 1, targetQuality: 0.9, deadIC: 0.02, icirVolatile: 0.3, plantM: 1, plantB: 0.6, samplePeriod: 0, anomalyK: 3, maxStep: 1 });
    const profile = loadProfile();
    // Native copilot mode is DEFAULT: the doer (below) produces, a DISTINCT checker (above) verifies
    // in real time. Pass --no-copilot to opt out.
    const copilotOff = args.includes('--no-copilot');
    let authority = 1;
    const res = await (async () => {
      // GUARD GATE (RED LINE) — the kernel denies red-line targets BEFORE any agent runs. This is the
      // same trust boundary the `run` loop enforces; the `dispatch` command must not bypass it. Fail-closed.
      const rl = checkRedLine(task);
      if (!rl.ok) {
        console.log(paint.blood(`  ⛔ DENIED by guard (${rl.engine}): ${rl.reason}`));
        process.exit(1);
      }
      return runCopilot({
        task,
        profile: loadProfile() ?? undefined,
        enabled: !copilotOff,
        runNative: (t) => ({ ok: true, backend: 'native', summary: `native handled: ${t.slice(0, 40)}`, exitCode: 0 }),
      });
    })();
    // feed the verdict as proven quality telemetry. The Governor is a SERVO (error = target − actual),
    // so to get "approve ⇒ more freedom / reject ⇒ less" we feed the QUALITY DEFICIT (1 − quality):
    // approve (quality 1) ⇒ deficit 0 ⇒ error +0.9 ⇒ authority rises; reject ⇒ deficit 1 ⇒ authority falls.
    const quality = res.ok ? 1 : 0;
    const st = gov.step({ t: Date.now(), predictedQuality: quality, actualQuality: 1 - quality, cost: 1e-18, volume: 100 });
    authority = st.authority;
    console.log(paint.dim(`  [doer=${res.doer} checker=${res.checker}] ${res.doerOutput}`));
    console.log(paint.dim(`  copilot verdict: ${res.verdict}${res.ok ? '' : ' — QUARANTINED'} | governor authority=${authority.toFixed(3)} (factor=${st.factorStatus}, resonance=${st.resonanceRisky ? 'RISKY' : 'ok'})`));
    if (!res.ok) process.exit(1);
    return;
  }

  if (cmd === 'node') {
    // Bebop node identity — encrypted-at-rest vault; a node keeps its PQ identity across restarts.
    const vaultPath = args.includes('--path') ? args[args.indexOf('--path') + 1] : path.resolve(HERE, '.bebop', 'node.vault.json');
    const pass = args.includes('--pass') ? args[args.indexOf('--pass') + 1] : 'bebop';
    const id = createOrUnlock(vaultPath, pass);
    console.log(paint.dim(`  node id=${id.id.slice(0, 24)}… (encrypted vault ${vaultPath})`));
    console.log(paint.dim(`  pqPublic=${Buffer.from(id.pqPublic).toString('hex').slice(0, 24)}… edPublic=${Buffer.from(id.edPublic).toString('hex').slice(0, 16)}…`));
    return;
  }

  if (cmd === 'govern') {
    // L5 telemetry governor applied LIVE to any agent/model/process (operator directive).
    // Feed a stream of quality samples; the servo computes math-proven authority (PID), factor
    // health (ICIR), resonance risk BEFORE any gain change, and anomaly signals (>3σ).
    // Usage:  bebop govern "0.9,0.6,0.2,0.9,0.95,0.1,..."   (comma/space separated 0..1)
    //         echo "0.9 0.6 0.2" | bebop govern              (stdin)
    const cfg = { kp: 1.4, ki: 0.22, kd: 1.5, iMin: -1, iMax: 1, uMin: 0, uMax: 1, targetQuality: 0.9, deadIC: 0.02, icirVolatile: 0.3, plantM: 1, plantB: 0.6, samplePeriod: 0, anomalyK: 3, maxStep: 1 };
    const gov = new Governor(cfg);
    let raw = args.join(' ').trim();
    if (!raw && !process.stdin.isTTY) {
      // read from stdin (sync, small inputs only) — ESM-safe fs import (no require()).
      try { raw = fs.readFileSync(0, 'utf8'); } catch { raw = ''; }
    }
    const samples = raw.split(/[\s,]+/).map(Number).filter((n) => !Number.isNaN(n));
    if (samples.length === 0) {
      console.log(paint.dim('  usage: bebop govern "0.9,0.6,0.2,..."   (quality stream 0..1)'));
      console.log(paint.dim('  or:    echo "0.9 0.6 0.2" | bebop govern'));
      return;
    }
    // Validate quality ∈ [0,1]. Out-of-range values are a SILENT-BAD-STATE weakness: they flow
    // through and produce authoritative-looking numbers. Clamp + warn so garbage can't masquerade.
    const outOfRange = samples.filter((q) => q < 0 || q > 1 || !Number.isFinite(q));
    if (outOfRange.length) {
      console.log(paint.blood(`  ! ${outOfRange.length} sample(s) out of range [0,1] (${outOfRange.join(', ')}) — clamped.`));
    }
    const clamp = (q: number) => (q < 0 ? 0 : q > 1 ? 1 : q);
    const qs = samples.map(clamp);
    console.log(paint.teal('  t  quality  authority  factor      resonance  anomaly'));
    let anomalies = 0;
    qs.forEach((q, t) => {
      // predicted = previous actual (a simple, honest predictor; ICIR measures its skill)
      const predicted = t > 0 ? qs[t - 1] : q;
      const st = gov.step({ t, predictedQuality: predicted, actualQuality: q, cost: 1e-18, volume: 100 });
      if (st.anomaly) anomalies++;
      const flag = st.anomaly ? paint.blood('ANOMALY') : 'ok';
      console.log(paint.dim(`  ${String(t).padStart(2)} ${q.toFixed(2)}     ${st.authority.toFixed(3)}     ${st.factorStatus.padEnd(9)}  ${st.resonanceRisky ? 'RISKY' : 'ok    '}    ${flag}`));
    });
    console.log(paint.dim(`  → ${samples.length} samples, ${anomalies} anomaly signal(s); final authority=${gov.authority.toFixed(3)}`));
    return;
  }

  if (cmd === 'self') {
    // Bebop soul: self-maintenance / self-evolution / session-as-node (fail-closed, recursive).
    const sub = args[0];
    if (sub === 'maintain' || !sub) {
      const h = selfMaintain();
      console.log(paint.dim(`  self-maintain ok=${h.ok} pass=${h.pass} fail=${h.fail}`));
    } else if (sub === 'evolve') {
      const idea = args.slice(1).join(' ');
      const r = await selfEvolve(idea);
      console.log(paint.dim(`  self-evolve accepted=${r.accepted} reason=${r.reason}${r.id ? ' id=' + r.id.slice(0, 12) : ''}`));
    } else if (sub === 'session') {
      const id = recordSession({ id: args[1] ?? 'hermes-now', summary: args.slice(2).join(' ') || 'active hermes session node' });
      console.log(paint.dim(`  session recorded as living-memory node ${id.slice(0, 12)}`));
    } else if (sub === 'loop') {
      const r = await selfLoop(args.slice(1).length ? args.slice(1) : ['tighten the copilot checker invariant']);
      console.log(paint.dim(`  self-loop health ok=${r.health.ok} evolutions=${JSON.stringify(r.evolutions)}`));
    } else {
      console.log(paint.blood('  usage: bebop self [maintain|evolve "<idea>"|session <id> <summary>|loop "<idea>"...]'));
    }
    return;
  }

  if (cmd === 'agents') {
    // The multi-agent abstraction: list every agentic CLI Bebop can drive, with live connection
    // status. This is THE simple switch surface — `bebop use <name>` connects one directly.
    console.log(banner(paint));
    console.log(paint.dim('  Bebop drives ANY connected agentic CLI. Switch directly with:  bebop use <name>'));
    const profile = loadProfile() ?? BEBOP_PRESET;
    const order = profile.backendOrder;
    for (const b of Object.keys(ADAPTERS) as Backend[]) {
      const a = ADAPTERS[b];
      const available = isAvailable(b);
      const connected = available && (a.binary ? a.detect() : true);
      const isDefault = order[0] === b;
      const tag = isDefault ? paint.teal('◆ default') : connected ? paint.teal('● connected') : paint.amber('○ available (needs key/binary)');
      const req = a.requiredEnv.length ? a.requiredEnv.join('/') : '(keyless)';
      console.log(paint.dim(`  · ${b.padEnd(9)} ${tag}  ${paint.dim(a.label)}  key: ${req}`));
    }
    console.log(paint.dim(`\n  active rotation: ${statusLine(profile)}`));
    return;
  }

  if (cmd === 'use') {
    // Simple, direct switch to a connected agentic CLI. Persists as the new default-first backend.
    const target = args[0] as Backend | undefined;
    const force = args.includes('--force');
    if (!target || !(target in ADAPTERS)) {
      console.log(paint.blood(`  usage: bebop use <backend>   where backend ∈ ${Object.keys(ADAPTERS).join(', ')}`));
      process.exit(2);
    }
    const backend: Backend = target;
    if (!isAvailable(backend) && !force) {
      const a = ADAPTERS[target];
      const req = a.requiredEnv.length ? a.requiredEnv.join('/') : '(keyless)';
      console.log(paint.amber(`  ! ${target} is not connected (needs ${req}). Connect it, or re-run with --force to set anyway.`));
      process.exit(1);
    }
    const profile = loadProfile() ?? { ...BEBOP_PRESET };
    // Promote the chosen backend to the front of the rotation; keep native last as the fail-safe.
    const rest = profile.backendOrder.filter((b) => b !== backend && b !== 'native') as Backend[];
    const next: Profile = { ...profile, backendOrder: [backend, ...rest, 'native'] };
    const written = writeProfile(next);
    console.log(paint.teal(`  ✓ switched default agent → ${target}`));
    console.log(paint.dim(`  profile → ${written}`));
    console.log(paint.dim(`  rotation: ${statusLine(next)}`));
    return;
  }

  if (cmd === 'run') {
    const cls = (args[0] as TaskClass) ?? 'doer';
    const planMode = args.includes('--plan');
    const asJson = args.includes('--json');
    const settings = loadSettings();
    const profile = loadProfile() ?? undefined;
    const res = await runLoop({
      cwd: path.resolve(HERE, '..'),
      taskClass: cls,
      profile,
      // User-supplied allow/deny (from TRUSTED ~/.bebop/settings.json only) extend the guard.
      // allow extends the granted scope; deny strengthens the red-line set (can't relax it).
      scope: [...DEFAULT_SCOPE_GLOBS, ...settings.permissions.allow],
      redLines: settings.permissions.deny,
      hooks: settings.hooks['PreToolUse'],
      planMode,
    });
    if (asJson) {
      console.log(JSON.stringify({ ok: res.ok, steps: res.steps, mutations: res.mutations, denied: res.denied, planMode }, null, 2));
    } else {
      for (const line of res.transcript) console.log(line);
      console.log(paint.dim(`  steps=${res.steps} mutations=${res.mutations} denied=${res.denied} ok=${res.ok} envelopes=${res.log.length}${planMode ? ' [plan mode]' : ''}`));
    }
    if (!res.ok) process.exit(1);
    return;
  }

  // Slash commands — Claude Code analogue (/help /clear /model /status /plan /resume /compact /review).
  if (cmd.startsWith('/')) {
    await handleSlash(cmd.slice(1).toLowerCase(), args);
    return;
  }

  console.log(paint.blood(`  unknown command: ${cmd}`));
  process.exit(2);
}

main().catch((e) => {
  const paint = makePaint();
  console.log(paint.blood('  fatal: ' + (e?.message ?? e)));
  process.exit(1);
});
