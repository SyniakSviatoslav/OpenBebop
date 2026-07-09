// src/integration/active-inference/loop-advisor.ts
//
// WIRING: Active Inference (FEP) as the loop's policy advisor.
//
// The loop.ts already has a `field.ts` 3-state directive (∇·F/∇×F). This module adds a SECOND,
// complementary advisor rooted in the Free-Energy Principle: given the agent's current belief over
// task states and its preferences (which outcomes it wants), pick the next loop action that minimizes
// Expected Free Energy. It is OFF unless `cfg.activeInference` is set (matches how `cfg.field` works).
//
// The two advisors are not in conflict: field = "where to look" (search geometry); active-inference =
// "what to do" (action selection under belief). The loop can use both.

import { selectPolicy, type PomdpModel } from './ai.ts';

export type LoopAction = 'explore' | 'act' | 'reflect' | 'done';

/**
 * Build a tiny POMDP for the agent loop and let Active Inference choose the next action.
 *  - states: {stuck, progressing, done}
 *  - obs:    noisy signal of state
 *  - actions (FEP): 0=explore (stay), 1=act (advance toward done), 2=reflect (recover stuck)
 *  - preferences C are log-domain; the caller expresses what it wants (e.g. reach 'done').
 *
 * `belief` is the loop's current confidence over [stuck, progressing, done] (must sum to 1).
 * Returns the chosen LoopAction. 'done' is emitted only when the best action already lands the
 * belief in the done state (it is a terminal, not a magic transition).
 */
export function adviseLoop(belief: number[], preferDone = true): LoopAction {
  if (belief.length !== 3) throw new Error('adviseLoop: belief must be length 3');
  const A = [
    [0.8, 0.15, 0.05], // obs0 likely from state0 (stuck)
    [0.15, 0.8, 0.05], // obs1 likely from state1 (progressing)
    [0.05, 0.1, 0.85], // obs2 likely from state2 (done)
  ];
  // B[action][to][from] = p(s'|s,a). Each column (fixed `from`) sums to 1.
  const B: number[][][] = [
    [[1, 0, 0], [0, 1, 0], [0, 0, 1]], // explore: stay
    [[0, 0, 0], [1, 0, 0], [0, 1, 1]], // act: stuck→progressing, progressing→done, done→done
    [[0, 0, 0], [1, 1, 0], [0, 0, 1]], // reflect: stuck→progressing, progressing stays, done stays
  ];
  // preference: want to be in 'done' (state2). log-domain: done high, others low.
  const C = preferDone ? [-3, -1, 2] : [0, 0, 0];
  const D = belief.slice();
  const model: PomdpModel = { A, B, C, D, actions: 3 };
  const { policy } = selectPolicy(model, 1, 1);
  const action = policy[0]; // 0=explore, 1=act, 2=reflect

  // Compute the post-action belief to decide if we've reached 'done'.
  const next: number[] = [0, 0, 0];
  for (let sp = 0; sp < 3; sp++) {
    for (let s = 0; s < 3; s++) next[sp] += belief[s] * B[action][sp][s];
  }
  if (next[2] > next[0] && next[2] >= next[1]) return 'done'; // belief landed in done
  return (['explore', 'act', 'reflect'] as const)[action];
}
