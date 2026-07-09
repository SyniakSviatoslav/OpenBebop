/**
 * re-loop.ts — Multi-pass reverse-engineering LOOP over the whole workspace.
 * Run: node --import tsx re-loop.ts
 * Produces RE-Loop-report.json with per-loop gaps + failure points.
 */
import { reverseEngineeringLoop, type ScanResult } from './src/integration/analytics/loop.ts';
import { writeFileSync } from 'node:fs';

type Roots = { path: string; prefix: string }[];

const selfRoots: Roots = [
  { path: '/root/bebop-repo/src/integration/analytics', prefix: 'analytics' },
];
const projectRoots: Roots = [
  { path: '/root/bebop-repo/src', prefix: 'bebop' },
  { path: '/root/dowiz/tools', prefix: 'dowiz-tools' },
  { path: '/root/dowiz/apps', prefix: 'dowiz-apps' },
];
const memoryRoots: Roots = [
  { path: '/root/.claude/projects/-root-dowiz/memory', prefix: 'lm' },
];
const toolsRoots: Roots = [
  { path: '/root/bebop-repo/src/integration/zenoh', prefix: 'bebop-zenoh' },
  { path: '/root/bebop-repo/src/integration/zkvm', prefix: 'bebop-zkvm' },
  { path: '/root/bebop-repo/src/integration/active-inference', prefix: 'bebop-ai' },
  { path: '/root/bebop-repo/src/integration/optical', prefix: 'bebop-optical' },
  { path: '/root/bebop-repo/src/integration/wetware', prefix: 'bebop-wetware' },
  { path: '/root/bebop-repo/src/integration/tigerbeetle', prefix: 'bebop-tb' },
  { path: '/root/bebop-repo/src/integration/analytics', prefix: 'bebop-analytics' },
];

function summarize(label: string, r: ScanResult & { drift?: { shift: number } }) {
  const topClusters = [...r.clusters].sort((a, b) => b.strength - a.strength).slice(0, 6);
  return {
    label,
    scanned: r.scanned,
    nodes: r.nodes.length,
    edges: r.adjacencyEdges,
    maxDegree: r.structure[3],
    spectralRadius: r.structure[4],
    drift: r.drift?.shift,
    couplingClusters: topClusters,
    isolatedCount: r.isolated.length,
    isolatedSample: r.isolated.slice(0, 12),
    cycle: r.cycle,
    crossEdgeCount: r.crossEdges.length,
    crossEdgeSample: r.crossEdges.slice(0, 10),
  };
}

const report: Record<string, unknown> = {};

// PASS 0 — RE on itself (the harness mines its own modules)
report.selfRE = summarize('SELF-RE (analytics harness on itself)', reverseEngineeringLoop({ roots: selfRoots }));

// LOOP 1 — whole project. No explicit cap: buildAdjacency uses a DYNAMIC cap
// (= min(moduleCount, HARD_EVD_CEILING)), so the full ~841-node graph is kept
// automatically and the 200+ "isolated" orphans from the old static cap=600 are gone.
report.loop1_wholeProject = summarize('LOOP1 whole project', reverseEngineeringLoop({ roots: projectRoots }));

// LOOP 2 — living memory corpus
report.loop2_livingMemory = summarize('LOOP2 living-memory corpus', reverseEngineeringLoop({ roots: memoryRoots }));

// LOOP 3 — cutting-edge integrated tools
report.loop3_integratedTools = summarize('LOOP3 integrated tools', reverseEngineeringLoop({ roots: toolsRoots }));

// GAPS / FAILURE-POINTS rollup
const gaps: string[] = [];
const self = report.selfRE as any;
if (self.isolatedCount > 0) gaps.push(`SELF-RE: ${self.isolatedCount} isolated analytics modules (dead code / unused export): ${self.isolatedSample.join(', ')}`);
if (self.cycle) gaps.push(`SELF-RE: circular dependency detected: ${self.cycle.join(' -> ')}`);
const lm = report.loop2_livingMemory as any;
if (lm.isolatedCount > 0) gaps.push(`LIVING-MEMORY: ${lm.isolatedCount} orphan notes (referenced by nothing / dangling wikilink): ${lm.isolatedSample.join(', ')}`);
const proj = report.loop1_wholeProject as any;
if (proj.crossEdgeCount > 0) gaps.push(`PROJECT: ${proj.crossEdgeCount} cross-namespace edges (vendored/duplicated code across repos): ${proj.crossEdgeSample.join(', ')}`);
if (proj.cycle) gaps.push(`PROJECT: circular dependency: ${proj.cycle.join(' -> ')}`);
report.gapsAndFailurePoints = gaps;

console.log(JSON.stringify(report, null, 2));
writeFileSync('/root/bebop-repo/RE-Loop-report.json', JSON.stringify(report, null, 2));
console.log('\n→ report written to /root/bebop-repo/RE-Loop-report.json');
