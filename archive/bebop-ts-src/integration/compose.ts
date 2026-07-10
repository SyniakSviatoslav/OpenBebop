// src/integration/compose.ts
//
// THE ASSEMBLED SOVEREIGN NODE — wires the reverse-engineered tools into one coherent runtime
// WITHOUT touching the deterministic kernel (kernel.ts stays pure; we compose around it).
//
// What this binds:
//   1. zkVM `decide()` journal  — every admitted command gets a tamper-evident digest over
//      (state serialization + commandHash + monotonic counter). The counter is the envelope seq,
//      which the kernel already supplies deterministically (no RNG).
//   2. TigerBeetle money boundary — money-tagged commands are checked by `moneyTransferChecker()`
//      (structural) and, on apply, run through `applyMoneyTransfer` conservation at shell time.
//   3. Active Inference advisor — `adviseLoop(belief)` selects the loop action under belief.
//
// The node is OFF-by-default for the optional pieces: the kernel journal is always recorded (it is
// pure + deterministic + free), but the money boundary and FEP advisor are engaged only when a
// `Ledger` / belief is supplied. This keeps the kernel's "identity/money is OUT OF KERNEL" discipline.

import {
  applyCommand,
  applyCommandChecked,
  commandHash,
  defaultChecker,
  genesis,
  type Command,
  type Envelope,
  type State,
} from '../kernel.ts';
import { verifyJournal } from './zkvm/kernel-journal.ts';
import { adviseLoop, type LoopAction } from './active-inference/loop-advisor.ts';

export interface JournalEntry {
  seq: number;
  cause: string; // command hash
  digest: Uint8Array; // zkVM decide() digest over state+command+counter
}

export interface NodeOptions {
  /** If provided, money-tagged commands are checked for structural validity (TigerBeetle). */
  money?: boolean;
  /** If provided, the node exposes an Active-Inference advisor over this belief. */
  belief?: number[];
}

/**
 * Apply a command through the Sovereign Node: the kernel's universal "above" gate
 * (zkVM tamper-evident journal, optionally composed with the TigerBeetle money boundary)
 * is the single path. This is a thin orchestration wrapper — the kernel decides, this records.
 *
 * Pure: counter = envelope seq (deterministic, supplied by the kernel).
 */
export function applyWithJournal(
  cmd: Command,
  state: State,
  opts: NodeOptions = {},
): { state: State; envelopes: Envelope[]; journal: JournalEntry; quarantined: boolean; reason?: string } {
  // The kernel is the ONE gate: it composes the zkVM journal (default on) + TB money boundary
  // (when opts.money) and returns the JOURNAL envelope + DENIED/quarantine as needed.
  const res = applyCommandChecked(cmd, state, defaultChecker, true, opts.money);
  const journalEnv = res.envelopes.find((e) => e.event.type === 'JOURNAL');
  const digest = journalEnv && journalEnv.event.type === 'JOURNAL'
    ? hexToBytes(journalEnv.event.digest)
    : new Uint8Array(0);
  const journal: JournalEntry = {
    seq: journalEnv ? journalEnv.seq : state.ingested.size,
    cause: commandHash(cmd),
    digest,
  };
  return { state: res.state, envelopes: res.envelopes, journal, quarantined: res.quarantined, reason: res.reason };
}

function hexToBytes(hex: string): Uint8Array {
  const out = new Uint8Array(hex.length / 2);
  for (let i = 0; i < out.length; i++) out[i] = parseInt(hex.slice(i * 2, i * 2 + 2), 16);
  return out;
}

/** Verify the whole journal against a replayed state sequence. Pure — falsifiable.
 *
 * Each journal entry binds (cumulative state after that command, commandHash, seq). We replay the
 * command sequence, recompute the expected digest at EACH step from the cumulative state, and compare
 * to the stored digest. Tampering ANY entry (or reordering) flips a digest → chain fails RED.
 */
export function verifyJournalChain(
  commands: Command[],
  journals: JournalEntry[],
  _finalState?: State,
): boolean {
  if (commands.length !== journals.length) return false;
  let st: State = genesis();
  for (let i = 0; i < commands.length; i++) {
    const cmd = commands[i];
    const cause = commandHash(cmd);
    if (journals[i].cause !== cause) return false; // cause binding
    const res = applyCommand(cmd, st);
    st = res.state;
    // counter = the seq of the last envelope produced by this command (its position in the log)
    const counter = res.envelopes.length ? res.envelopes[res.envelopes.length - 1].seq : st.ingested.size;
    if (!verifyJournal(st, cause, counter, journals[i].digest)) return false; // tamper-evident
  }
  return true;
}

/** Active-Inference advisory: pick the next loop action under the supplied belief. */
export function advise(belief: number[]): LoopAction {
  return adviseLoop(belief);
}
