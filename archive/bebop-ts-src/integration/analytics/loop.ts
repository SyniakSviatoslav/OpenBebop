/**
 * loop.ts — Next-level reverse-engineering LOOP for the whole workspace.
 *
 * v2: scans bebop (src + integration tools), dowiz (tools + apps), and the
 * living-memory corpus (markdown wikilinks), building a NAMESPACED adjacency
 * (no false cross-repo merges), then surfaces latent coupling clusters AND
 * gap detectors (isolated / circular / ambiguous). Optional architecture-drift
 * between two scans. Deterministic, bounded, CI-safe.
 */

import { readFileSync, readdirSync, statSync } from 'node:fs';
import { join, resolve } from 'node:path';
import {
  buildAdjacency,
  couplingClusters,
  architectureDrift,
  isolatedNodes,
  findCycle,
} from './arch-mine.ts';
import type { Mat } from './matrix.ts';

export interface ScanResult {
  scanned: number;
  nodes: string[];
  adjacencyEdges: number;
  clusters: { strength: number; members: string[] }[];
  isolated: string[];
  cycle: string[] | null;
  crossEdges: [string, string][];
  structure: number[];
  A: Mat;
}

/** Recursively collect .ts/.tsx/.md files under root, skip node_modules/dist/.git. */
function walk(root: string, prefix: string, cap = 4000): { id: string; file: string; isMarkdown: boolean }[] {
  const out: { id: string; file: string; isMarkdown: boolean }[] = [];
  const stack = [root];
  while (stack.length && out.length < cap) {
    const dir = stack.pop()!;
    let entries: string[];
    try { entries = readdirSync(dir); } catch { continue; }
    for (const e of entries) {
      if (e === 'node_modules' || e === 'dist' || e === '.git' || e === '.turbo') continue;
      const full = join(dir, e);
      let st; try { st = statSync(full); } catch { continue; }
      if (st.isDirectory()) stack.push(full);
      else if (/\.(ts|tsx|md)$/.test(e)) {
        const rel = resolve(full).replace(resolve(root), '');
        const isMd = /\.md$/.test(e);
        const id = `${prefix}:${rel.replace(/^\//, '').replace(/\.(ts|tsx|md)$/, '')}`;
        out.push({ id, file: full, isMarkdown: isMd });
      }
    }
  }
  return out;
}

function readSrc(file: string): string {
  try { return readFileSync(file, 'utf8'); } catch { return ''; }
}

export function scanProjects(roots: { path: string; prefix: string }[], cap?: number): ScanResult {
  const modules: { id: string; source: string; isMarkdown?: boolean }[] = [];
  for (const r of roots) for (const m of walk(r.path, r.prefix)) modules.push({ id: m.id, source: readSrc(m.file), isMarkdown: m.isMarkdown });
  const { nodes, A, crossEdges } = buildAdjacency(modules, { cap });
  let edges = 0;
  for (const row of A) for (const v of row) edges += v;
  edges /= 2;
  const clusters = couplingClusters({ nodes, A });
  const isolated = isolatedNodes({ nodes, A });
  const cycle = findCycle({ nodes, A });
  let maxDeg = 0;
  for (let i = 0; i < nodes.length; i++) { let d = 0; for (let j = 0; j < nodes.length; j++) d += A[i][j]; if (d > maxDeg) maxDeg = d; }
  const structure = [nodes.length, edges, nodes.length ? edges / nodes.length : 0, maxDeg, clusters[0]?.strength ?? 0];
  return { scanned: modules.length, nodes, adjacencyEdges: edges, clusters, isolated, cycle, crossEdges, structure, A };
}

export function reverseEngineeringLoop(opts: {
  roots: { path: string; prefix: string }[];
  prevAdj?: { nodes: string[]; A: Mat };
  cap?: number;
}): ScanResult & { drift?: { shift: number } } {
  const cur = scanProjects(opts.roots, opts.cap);
  const result: ScanResult & { drift?: { shift: number } } = { ...cur };
  if (opts.prevAdj) {
    const d = architectureDrift(opts.prevAdj, { nodes: cur.nodes, A: cur.A });
    result.drift = { shift: d.shift };
  }
  return result;
}
