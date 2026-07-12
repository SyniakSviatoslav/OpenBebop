# Escalations — human-arbitrated truth resolutions

When `scripts/logic-gate.mjs` (Enforcement model §0 of `LOGIC-LAWS.md`) cannot
establish that a claim is true — because it is **unbacked**, **self-referential
(paradox)**, or **silently assumes LEM** in a non-classical context — it writes
an entry here and returns exit code `2` (commit allowed, but tracked). A human
arbiter (the operator or a designated user) fills the `Resolution` field.

**Rules**
- `OPEN` escalations may ship, but must be resolved before a release cut.
- Resolution values: `TRUE — <ref>`, `FALSE`, `DEFER — <reason>`.
- Never delete an `OPEN` entry to make the gate green. That itself is a
  non-contradiction violation (hiding a claim) and will be caught.

---

<!-- New ESC entries are appended by logic-gate.mjs above this line. -->
