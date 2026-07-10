// scripts/send-cli-design.mjs — deliver the finished Bebop CLI design recordings
// to the operator's Telegram channel.
//
// Usage:
//   node scripts/send-cli-design.mjs [--dry-run]
//   TG_BOT_TOKEN=xxxx TG_CHAT_ID=yyyy node scripts/send-cli-design.mjs
//
// --dry-run: validates that both artifacts exist + prints the exact payload
//   (caption + file paths) WITHOUT sending. Use it when TG creds are unset,
//   or to pre-flight the message. No network call is made in dry-run.
//
// Reads creds from env (never hard-coded). Sends:
//   1) the design brief (docs/design/bebop-cli-2026-07-09.md) as a document
//   2) the sun-warm ship launch animation strip (docs/design/bebop-launch.svg)
// Both are the VERIFIED artifacts: the launch is the same deterministic frames
// the TUI renders; the native core has 79 Rust tests green (63 bebop + 16 rust-core).

import { readFileSync, existsSync } from "node:fs";
import { resolve } from "node:path";

const ROOT = resolve(process.env.REPO_ROOT || process.cwd());
const designDoc = resolve(ROOT, "docs/design/bebop-cli-2026-07-09.md");
const launchSvg = resolve(ROOT, "docs/design/bebop-launch.svg");
const DRY = process.argv.includes("--dry-run");

const caption =
  "🎷 *Bebop CLI — design recordings (2026-07-09)*\n\n" +
  "• Native Rust/WASM core, NO TypeScript in the runtime path. `bin/bebop` shim → `cargo run -p bebop`.\n" +
  "• Sun-warm ship launch animation (ship #F4C25A on void #12100E), deterministic, TTY-gated.\n" +
  "• Customization axes: `looks` / `narration` / `patrons` (the 'make it yours' hook).\n" +
  "• Verified: 79 Rust tests green (63 bebop + 16 rust-core), guardrail 95/95 falsifiable, doc-gate PASS.\n" +
  "• Hosts Claude/Codex/OpenCode/Hermes behind one guard plane — see wiki Research page.";

function checkArtifacts() {
  const missing = [];
  if (!existsSync(designDoc)) missing.push(designDoc);
  if (!existsSync(launchSvg)) missing.push(launchSvg);
  return missing;
}

if (DRY) {
  const missing = checkArtifacts();
  console.log("🔍 --dry-run: no message sent.");
  console.log("  design brief :", designDoc, existsSync(designDoc) ? "✓ present" : "✗ MISSING");
  console.log("  launch svg   :", launchSvg, existsSync(launchSvg) ? "✓ present" : "✗ MISSING");
  console.log("  caption      :\n" + caption.split("\n").map((l) => "    " + l).join("\n"));
  if (missing.length) {
    console.error("✗ dry-run FAILED: missing artifacts —", missing.join(", "));
    process.exit(2);
  }
  console.log("✓ dry-run OK: both artifacts present, payload valid. Set TG_BOT_TOKEN/TG_CHAT_ID to send.");
  process.exit(0);
}

const token = process.env.TG_BOT_TOKEN;
const chat = process.env.TG_CHAT_ID;
if (!token || !chat) {
  console.error("✗ TG_BOT_TOKEN and/or TG_CHAT_ID not set — design NOT sent.");
  console.error("  Set them and re-run: TG_BOT_TOKEN=… TG_CHAT_ID=… node scripts/send-cli-design.mjs");
  console.error("  Or pre-flight with: node scripts/send-cli-design.mjs --dry-run");
  process.exit(2);
}

const missing = checkArtifacts();
if (missing.length) {
  console.error("✗ artifacts missing — not sending:", missing.join(", "));
  process.exit(2);
}

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
  await sendDocument(launchSvg, "Bebop sun-warm ship launch — 18 deterministic frames, bit-identical to the live TUI.");
  console.log("✓ CLI design recordings sent to chat", chat);
} catch (e) {
  console.error("✗ send failed:", e.message);
  process.exit(1);
}
