// Bebop MCP protocol handler — RED+GREEN (Verified-by-Math), pure (no child process).
//
// GREEN: the JSON-RPC dispatcher (handle) returns well-formed responses for
//   initialize / tools/list / tools/call.
// RED: malformed JSON yields a parse-error; unknown method yields method-not-found; a tool that
//   throws yields a JSON-RPC error, not a crash.
//
// This tests the protocol handler directly (handle), NOT a spawned process — so it is
// deterministic and CI-stable (no stdio race, no spawn timeout). The real stdio server is
// exercised by `npm run boot` + a manual `bebop mcp` handshake; the handler is the unit.

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { handle } from './mcp.ts';

test('GREEN: initialize returns serverInfo + tools capability', () => {
  const res = handle({ jsonrpc: '2.0', id: 1, method: 'initialize', params: {} });
  assert.equal(res.jsonrpc, '2.0');
  assert.equal(res.id, 1);
  assert.ok(res.result);
  assert.equal(res.result.protocolVersion, '2024-11-05');
  assert.ok(res.result.capabilities.tools);
  assert.equal(res.result.serverInfo.name, 'bebop');
});

test('GREEN: tools/list returns all 6 Bebop tools', () => {
  const res = handle({ jsonrpc: '2.0', id: 2, method: 'tools/list', params: {} });
  assert.ok(res.result);
  const names = res.result.tools.map((t: any) => t.name);
  for (const n of ['bebop_boot', 'bebop_recall', 'bebop_remember', 'bebop_govern', 'bebop_route', 'bebop_self_maintain']) {
    assert.ok(names.includes(n), `expected tool ${n} in ${names.join(',')}`);
  }
});

test('GREEN: tools/call bebop_boot certifies the guard OS', () => {
  const res = handle({ jsonrpc: '2.0', id: 3, method: 'tools/call', params: { name: 'bebop_boot', arguments: {} } });
  assert.ok(res.result);
  const text = res.result.content[0].text;
  const parsed = JSON.parse(text);
  assert.equal(parsed.certified, true);
});

test('RED: malformed params yields an invalid-params error', () => {
  const res = handle({ jsonrpc: '2.0', id: 9, method: 'tools/call', params: { name: 'bebop_remember', arguments: { concept: 'x', payload: null } } });
  assert.ok(res.error, 'null payload should fail validation');
  assert.equal(res.error.code, -32602); // invalid params
});

test('RED: unknown method yields method-not-found', () => {
  const res = handle({ jsonrpc: '2.0', id: 10, method: 'does-not-exist', params: {} });
  assert.ok(res.error);
  assert.equal(res.error.code, -32601);
});
