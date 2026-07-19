# Red-line ledger — errors & suggestions routed to the planning team

**Purpose.** When work on a red-line area (auth / money / RLS / migrations / bulk-edit /
crypto-constant / wire-schema) is executed on autopilot strictly against a roadmap plan or
blueprint (AGENTS.md §8), the executing agent has **zero license to invent, discover, or
improvise** (жодної самодіяльності). The blueprint's exact schema is followed **byte for byte**.

Any error encountered, ambiguity found, or improvement suggested is **NOT acted on** by the
executing agent. It is written here, in this ledger, and left for the planning team to
adjudicate. The executing agent then continues only with the parts of the blueprint that remain
unambiguous and byte-exact — or stops if the blocker is load-bearing.

**This ledger is append-only by convention (not yet a mechanical gate — see §6 for the
enforcement bar this should eventually meet). Never delete or rewrite a prior entry — the
planning team owns it.**

---

## Entry format

```
### <YYYY-MM-DD> — <red-line area> — <blueprint/roadmap ref>
- KIND: error | ambiguity | suggestion
- WHERE: <file:line or schema field, byte offset if applicable>
- BLUEPRINT SAYS: <the exact spec text / schema bytes the agent was following>
- OBSERVED: <what the live code / registry / test actually showed — verbatim, no interpretation>
- IMPACT: <does this block byte-exact execution, or is it advisory?>
- AGENT ACTION: deferred to planning team (no code written for this item)
- STATUS: open  # planning team flips to: accepted | rejected | scheduled <ref>
```

---

## Entries

<!-- append below; do not edit above this line -->
