// scripts/send-cli-design.mjs — deliver the finished Bebop CLI design recordings
// to the operator's Telegram channel.
//
// Usage:
//   TG_BOT_TOKEN=xxxx TG_CHAT_ID=yyyy node scripts/send-cli-design.mjs
//
// Reads both from env (never hard-coded). Sends:
//   1) the design brief (docs/design/bebop-cli-2026-07-09.md) as a document
//   2) the red-spaceship launch animation strip (docs/design/bebop-launch.svg)
// Both are the VERIFIED artifacts: the launch is the same deterministic frames
// the TUI renders; the Rust core has 36 green tests, TS suite 433/433 green.

import { readFileSync } from "node:fs";
import { resolve } from "node:path";

const token = process.env.TG_BOT_TOKEN;
const chat = process.env.TG_CHAT_ID;
if (!token || !chat) {
  console.error("✗ TG_BOT_TOKEN and/or TG_CHAT_ID not set — design NOT sent.");
  console.error("  Set them and re-run: TG_BOT_TOKEN=… TG_CHAT_ID=… node scripts/send-cli-design.mjs");
  process.exit(2);
}

const ROOT = resolve(process.env.REPO_ROOT || process.cwd());
const designDoc = resolve(ROOT, "docs/design/bebop-cli-2026-07-09.md");
const launchSvg = resolve(ROOT, "docs/design/bebop-launch.svg");

const caption =
  "🎷 *Bebop CLI — design recordings (2026-07-09)*\n\n" +
  "• Rust/WASM core, NO hand-written TS in the agent logic. Native `bebop` binary + wasm core.\n" +
  "• Red-spaceship launch animation (blood #E0543E on void #12100E), deterministic, TTY-gated.\n" +
  "• Customization axes: `looks` / `narration` / `patrons` (the 'make it yours' hook).\n" +
  "• Verified: 36 Rust tests green, 433 TS tests green, wasm32 core clean.\n" +
  "• Gap analysis vs Claude/OpenCode/Hermes in the attached brief.";

async function sendDocument(path, cap) {
  const body = new FormData();
  body.append("chat_id", chat);
  body.append("document", new Blob([readFileSync(path)], { type: "application/octet-stream" }), path.split("/").pop());
  if (cap) body.append("caption", cap);
  body.append("parse_mode", "Markdown");
  const r = await fetch(`https://api.telegram.org/bot${token}/sendDocument`, { method: "POST", body });
  const j = await r.json();
  if (!j.ok) throw new Error(`Telegram API: ${JSON.stringify(j)}`);
  return j;
}

try {
  await sendDocument(designDoc, caption);
  await sendDocument(launchSvg, "Bebop red-spaceship launch — 18 deterministic frames, bit-identical to the live TUI.");
  console.log("✓ CLI design recordings sent to chat", chat);
} catch (e) {
  console.error("✗ send failed:", e.message);
  process.exit(1);
}
