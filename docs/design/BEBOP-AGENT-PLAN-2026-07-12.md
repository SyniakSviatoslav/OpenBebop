# BEBOP AGENT — DEEP RESEARCH + PLAN OF CHANGES (2026-07-12)

> Статус: ДОСЛІДЖЕННЯ + АНАЛІЗ + ПРОПОЗИЦІЯ. Не імплементація (правило оператора:
> push-plans-first, build лише після підтвердження). Цей документ — джерело правди
> про БАЖАНИЙ стан; живий репозиторій = що Є (див. GROUND-TRUTH нижче).

## 0. Метод
Спершу зібрано реальний стан репозиторію (`grep`/читання модулів), потім нет-практики
(OpenTelemetry GenAI, cargo-deny, OpenBao/SOPS, Keep/Alertmanager, Mem0/agentmemory,
GOAP/PID-сервови, game-UX sliders), потім — пропозиція змін по 17 пунктах оператора.
Кожен пункт має: що Є → що треба → open-source precedent → дефолт/авто значення.

## 1. ЩО ВЖЕ Є В РЕПО (ground-truth, не вигадане)
- `tui.rs` (1285 рядків): структура `Telemetry` уже має `drift: Vec<u64>`,
  `quality: Vec<u64>`, `cost: Vec<u64>`, `feed: Vec<(AgentState,String)>` (live log),
  `hints`, `twin: Option<Box<Telemetry>>` (What-If fork). Тобто дрейф/якість/вартість
  і "близнюк" для порівняння вже закладені — треба лише ВИВЕСТИ їх у панель + дати
  історію (rolling window), а не лише LCG-симуляцію.
- `analytics.rs`: Kalman `kalman1d_step` + `kalman_anomaly` (інновація/сигма) — це і є
  детектор дрейфу точності/регресії. `dual_track_gate` (пропозиція vs Truth Layer) —
  анти-галюцинація. `goap::plan` — символьний планувальник.
- `governor.rs`: `GovConfig { kp, ki, kd, i_min, i_max, u_min, u_max, target_quality,
  dead_ic }` + `GovState { authority, ... }`. Це ЕТАЛОН того, як "глобальне правило =
  набір коефіцієнтів із діапазоном". Треба узагальнити цей патерн на ВСІ глобальні
  правила/хуки (єдиний `LawConfig` registry).
- `stabilizer.rs`: Lyapunov `adaptation_allowed(v_dot, freeze_threshold)`,
  `sliding_surface`, `smc_control`, `potential_well` — це механізми "динамічної
  корекції під навантаження". Пряме підґрунтя для "дефолтно увімкнено + авто-коригування
  під ресурси".
- `vault.rs`: PQ-гібридний (ML-KEM-768 ⊕ X25519, ML-DSA-65 ⊕ Ed25519) сховища секретів,
  Argon2id, XChaCha20-Poly1305, self-certifying identity. Це ВЖЕ "один vault для секретів".
  Треба: розподіл по папках (scopes) + env експорт.
- `memory.rs`: `LivingMemory` із `attic` (non-destructive eviction) + `tick()`. Є
  `restore()`. Треба: TTL записів, configurable forgetting on/off, snapshots/backups.
- `audit.rs`, `agentic_git.rs`, `multipilot.rs` (multi-pilot = субагентство/оркестрація!),
  `research_patterns.rs` (`route_model`), `router.rs`, `recall_graph.rs`, `knowledge.rs`,
  `customize.rs` (профілі/looks), `mcp.rs`. Субагентність/оркестрація/маршрутизація моделей
  ВЖЕ є як модулі — треба їх зв'язати в одну видиму конфігурацію + CLI-видимість.
- `law-hooks.mjs` + `logic-gate.mjs` + `deny.toml` + `ci-supply-chain.sh`: хуки уже
  підключені до pre-commit і СПРАЦЮВАЛИ (знайшли реальний борг). Це підтверджує, що
  "правила/хуки = практика, не декларація" працює.

## 2. НЕТ-ПРАКТИКИ (open-source, перевірені)
- **Телеметрія агентів**: OpenTelemetry GenAI Semantic Conventions (2025–2026) —
  стандарт spans/metrics для LLM (model, input/output tokens, latency, cost). Braintrust/
  Traceloop openllmetry — OSS інструментування. Висновок: не винаходити свій формат;
  емітити OTel-сумісні метрики (trace per agent action), експортер — плагінний.
- **Дрейф/регресія**: Kalman-фільтр (у нас уже є) + контрольні карти (Shewhart) —
  класичний SRE-підхід "anomaly = innovation > k·sigma". Наш `kalman_anomaly` уже це робить.
- **Секрети**: OpenBao (Linux Foundation fork of Vault) / SOPS+age — OSS, self-host.
  Розподіл по scope (per-folder) = SOPS `--encrypted-regex` або Vault KV v2 paths.
- **Сповіщення**: Keep (AIOps, OSS) / Prometheus Alertmanager — декларативні YAML-тригери,
  webhook до Slack/Telegram. Легко підключити як плагін експортера.
- **Living memory / забування**: Mem0 (OSS) — decay + eviction; agentmemory (OSS) — TTL +
  snapshot. Наш `attic` уже non-destructive — розширити TTL + snapshot.
- **Settings UX (game design)**: NN/groups Slider rules — слайдери добрі для НЕПЕРЕРВНИХ
  діапазонів, але погані для точних значень (користувач не влучить у 0.37). Кращий патерн:
  presets (Low/Med/High/Max-Eco) + слайдер із числовим полем + "reset to default/auto".
  Показувати поточний вплив (live preview) — як у налаштуваннях гри показують "Damage +12%".
- **Token policy / авто-ротація**: гібридна стратегія (cheap gatekeeper → expensive
  reasoner), prompt-caching, budget caps. `research_patterns::route_model` уже маршрутизує
  cheap/expensive — розширити до явного budget + fallback ladder.
- **Chain-of-thought у CLI**: OpenCode `feed` + Hermes hints — live working log + контекстні
  підказки. Наш `Telemetry.feed`/`hints` уже це моделюють.
- **Audit/history/replay**: agentmemory (12 hooks + MCP) — перехоплення дій агента у
  таймлайн; Nylas audit CLI — одна команда записує всі CLI-дії. Наш `audit.rs` +
  `agentic_git.rs` — розширити до unified ActionLog із replay.

## 3. ПРОПОЗИЦІЯ ЗМІН ПО 17 ПУНКТАХ

> Формат: [P#] пункт → ЩО Є → ЩО ЗРОБИТИ → precedent → дефолт/авто.

**[P1] Детальна телеметрія: дрейф точності, регресії, порівняння швидкості**
- Є: `Telemetry.drift/quality/cost` (LCG-симуляція) + `analytics::kalman_anomaly`.
- Зробити: замінити LCG-симуляцію на РЕАЛЬНІ rolling windows із історії промптів/лурів;
  Kalman-згладжування серії; детектор регресії = `kalman_anomaly` на похідній latency/quality.
  Панель TUI: додати вкладку "Telemetry" із sparkline дрейфу + порівняння "поточний промпт
  vs попередній" (delta%). 
- Precedent: OTel GenAI + Shewhart control charts.
- Дефолт: увімкнено; window=50 останніх дій; k=3.0 сигма.

**[P2] Auto-дослідження нету + тулів/ліб, OpenCode/Hermes-патерни**
- Є: `research_patterns.rs`, `enrich.rs` (README згадує enrichment).
- Зробити: фоновий "research daemon" (cron-подібний, уже є `crons` у Hermes-навігації
  оператора) що періодично: (a) шукає нові версії залежостей/адвізорі; (b) звіряє з
  OpenCode/Hermes best-practices; (c) генерує ENRICHMENT-пропозицію (doc + skill + MR).
- Precedent: Hermes self-improvement curator + Osmani "Ralph Wiggum loop".
- Дефолт: увімкнено; період=24h; спливає як proposal, не auto-merge.

**[P3] Auto-reverse-engineering + впровадження findings на весь проєкт**
- Є: `law-hooks.mjs` (перевіряє практикою), `verify-mcp-stdio`/`parallel-rust` скіли.
- Зробити: коли P2 знаходить finding → авто-генерується патч + RED+GREEN тест (за
  патерном `falsifiable`), проганяється через `law-hooks` gate; лише якщо GREEN — він
  стає proposal на розгляд. Ніякого "тихого" мержу.
- Precedent: наш же `guardrail-falsifiable-proof.mjs`.
- Дефолт: увімкнено; require_red_green=true.

**[P4] Глобальні правила/хуки — конфігуровані діапазони + коефіцієнт**
- Є: `governor::GovConfig` (еталон). `law-hooks.mjs` hard-codes пороги.
- Зробити: єдиний `LawConfig` (TOML/YAML) registry — кожне правило = `{ id, enabled,
  coefficient, min, max, auto }`. `law-hooks.mjs` читає з registry, не з коду. Хуки стають
  data-driven.
- Precedent: governor PID + cargo-deny config.
- Дефолт: усі enabled; coefficient=значення з blueprint; auto=true.

**[P5] Правила/хуки відкриті до ручного переналаштування, зручно**
- Є: `customize::Profile` (looks).
- Зробити: TUI-вкладка "Laws" + CLI `bebop laws set <id> <coef>` + `--interactive`
  редактор. Зміни пишуться у `LawConfig` (versioned, git-tracked). Кожне правило має
  inline help ("що порушує, що робить").
- Precedent: game-UX presets + numeric field + live preview.
- Дефолт: editable; preset="Balanced".

**[P6] Settings UX з game-design (найкращі приклади)**
- Зробити: presets (Eco/Balanced/Max-EV/Paranoid) + слайдер+число + reset-to-default/auto
  + live-impact preview ("Auth drift alert @ k=2.5 → 1 alert/1000 actions"). Слайдери лише
  для неперервних; для булевих — toggle; для вибору — segmented control.
- Precedent: NN/groups slider rules + game settings UX bible.
- Дефолт: Balanced preset; auto-tuning on.

**[P7] Авто-дослідження + enrichment для нових фіч (пропозиція на розгляд)**
- Об'єднує P2+P3 для фіч: при створенні фічі агент авто-шукає нет/репо-аналоги, пише
  ENRICHMENT-пропозицію (doc + skill + MR). Як і P3 — лише proposal.
- Дефолт: увімкнено для feature-гілок.

**[P8] Історія дій агентів — аналіз/телеметрія/хронологія + replay**
- Є: `audit.rs`, `agentic_git.rs`.
- Зробити: unified `ActionLog` (append-only, structured: {ts_monotonic, agent, action,
  args, result, cot}). Команда `bebop log --timeline` (TUI-хронологія) + `bebop replay
  <id>` (переграти дію). Зберігається у vault-scoped файл.
- Precedent: Nylas audit CLI + agentmemory 12 hooks.
- Дефолт: увімкнено; retention=30d; replay read-only.

**[P9] Видимий у CLI chain-of-thought**
- Є: `Telemetry.feed`/`hints`.
- Зробити: `bebop run --show-cot` виводить live feed + hints + поточний `AgentState`
  (booting/thinking/…). У TUI — панель "Thought" із karaoke-ефектом (уже є `karaoke`).
- Дефолт: увімкнено у TUI; у CLI — за прапором (шум).

**[P10] Субагентство + оркестрація паралельних сесій**
- Є: `multipilot.rs` (multi-pilot), `research_patterns.rs`, `router.rs`.
- Зробити: orchestrator API (`spawn_subagent(goal, role)`, `join_all`), паралельні
  worktrees (як у цій сесії), результат — як пропозиції. Видимість: TUI "Swarm" вкладка
  (кожен субагент = свій `Telemetry` + `AgentState`).
- Precedent: ClawArena-Team subagent orchestration + наша worktree-практика.
- Дефолт: увімкнено; max_concurrent=3; red-line дії (auth/money) → людина.

**[P11] Усе налаштовуване + видиме у налаштуваннях, із дефолт + авто значеннями**
- Узагальнення P4–P10: єдиний `Settings` об'єкт (config + runtime derived). TUI "Settings"
  показує кожне поле: value / default / auto-derived / source (user|auto|blueprint).
- Дефолт: auto-derived пріоритетні, якщо користувач не перевизначив.

**[P12] Дефолтно увімкнено + динамічна корекція під навантаження**
- Є: `stabilizer::adaptation_allowed` + `governor`.
- Зробити: feedback-цикл — при high load (CPU/context%) авто-знижує parallelism/concurrency,
  при low — підвищує. Коефіцієнти з P4. Fail-closed: якщо метрика недоступна — повертає до
  дефолту.
- Precedent: stabilizer Lyapunov + governor PID.
- Дефолт: on; freeze_threshold=0.05.

**[P13] Token spend policy — макс економія дефолтно, крім reasoning/review/research**
- Є: `research_patterns::route_model(cheap_adequate, budget_left, cheap_cost)`.
- Зробити: `TokenPolicy { default_tier: "cheap", elevated: [reasoning, review, research],
  budget_cap, fallback_ladder: [cheap→mid→exp] }`. Економія = default cheap; підвищення
  лише для 3 ролей. Fallback при помилці/таймауті моделі.
- Precedent: hybrid LLM cost strategy (gatekeeper + reasoner).
- Дефолт: economy=true; elevated roles як вище; cap=soft.

**[P14] Авто-ротація моделей, фолбеки**
- Є: `route_model`, `router.rs`.
- Зробити: `ModelRegistry` із health + latency; при збою — наступна у ladder. Експоненційна
  затримка. Лог ротацій у ActionLog.
- Precedent: наш же `router.rs` + SRE retry.
- Дефолт: on; max_retries=3; backoff=2^n.

**[P15] Легке підключення сторонніх телеметрії/сповіщень**
- Зробити: плагінний `Exporter` trait (OTel push, webhook, file). `Notifier` trait
  (Slack/Telegram/webhook). Конфіг у `Settings`.
- Precedent: OTel exporters + Keep/Alertmanager.
- Дефолт: exporter=local-file; notifier=none (opt-in).

**[P16] Один vault для секретів/env, розподіл по папках**
- Є: `vault.rs` (PQ-hybrid, self-certifying).
- Зробити: `vault mount <scope> <path>` → scope-bound env (як Vault KV v2 paths / SOPS
  per-dir). `bebop env --scope=dev` експортує лише той scope. Шифрування як у vault.rs.
- Precedent: OpenBao scopes + SOPS encrypted-regex.
- Дефолт: один vault; scopes=[] (все у global).

**[P17] Сповіщення/тригери на регресію агентів і розробки + контроль регресії коду**
- Зробити: `AlertRule { metric, op, threshold, channel }` (як Prometheus). Тригери:
  (a) агентська регресія — дрейф якості/дрейф телеметрії (P1); (b) регресія коду —
  `cargo test` count падає, clippy errors росте, deny/advisory з'являється. Вже є
  `law-hooks` gate — обгорнути його у AlertRule. Сповіщення через P15.
- Precedent: Alertmanager + наш supply-chain gate.
- Дефолт: alert on code-regression (hard) + agent-drift (warn); channel=local log.

## 4. ЖИТТЄВИЙ ЦИКЛ LIVING MEMORY (додаток до P-ів)
- Є: `memory.rs` `attic` (non-destructive) + `tick()`.
- Зробити:
  - `forgetting_enabled: bool` (P-оператор хоче вмикати/вимикати забування) — дефолт on.
  - `ttl: Option<u64>` на запис (за замовчуванням за Layer: Working=short, Long=long).
  - `snapshot()` / `restore_snapshot(path)` — серіалізація nodes+attic у vault-scoped файл.
  - `backup()` авто перед кожним `tick` (attic уже це робить, розширити до файлу).
- Precedent: Mem0 decay + agentmemory TTL/snapshot.
- Дефолт: forgetting=on; ttl за Layer; snapshot перед tick.

## 5. КРОКИ ВПРОВАДЖЕННЯ (після підтвердження оператора)
1. `LawConfig` registry (P4) — foundation для P5/P6/P11/P12.
2. Telemetry real rolling windows + TUI вкладка (P1).
3. ActionLog + timeline/replay (P8/P9).
4. Exporter/Notifier traits + AlertRule (P15/P17).
5. TokenPolicy + ModelRegistry rotation (P13/P14).
6. Vault scopes (P16).
7. Research daemon + enrichment proposal loop (P2/P3/P7).
8. Multipilot orchestrator visibility (P10).
9. Living-memory TTL/snapshot (§4).
Кожен крок — свій branch → law-hooks gate (RED+GREEN) → proposal → merge.

## 6. ЩО НЕ ТРЕБА (YAGNI, за AGENTS.md)
- Не будуємо власний формат телеметрії (є OTel).
- Не будуємо власний secrets-manager (є vault.rs + OpenBao/SOPS як reference).
- Не auto-мержимо findings (лише proposal — безпека/red-line).
- Не додаємо UI-фреймворк (є ratatui TUI).
