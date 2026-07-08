// Bebop MCP server — RED+GREEN (Verified-by-Math).
//
// GREEN: an MCP client handshake (initialize -> tools/list -> tools/call) returns a well-formed
//   JSON-RPC response and a tool result.
// RED: a malformed JSON line yields a parse-error response; an unknown method yields method-not-found.

import assert from 'node:assert/strict';
import test from 'node:test';
import { spawn } from 'node:child_process';

function rpc(input: { jsonrpc: '2.0'; id?: number | string | null; method: string; params?: any }): Promise<any> {
  return new Promise((resolve, reject) => {
    const proc = spawn(process.execPath, ['bebop.ts', 'mcp'], { cwd: process.cwd(), stdio: ['pipe', 'pipe', 'inherit'] });
    let out = '';
    proc.stdout.on('data', (d) => {
      out += d.toString();
      // Parse line-by-line; resolve on the first complete JSON-RPC response.
      const lines = out.split('\n');
      for (const line of lines) {
        const t = line.trim();
        if (!t) continue;
        try {
          const msg = JSON.parse(t);
          // Resolve on any valid JSON-RPC response (result OR error) matching our id.
          if (msg && msg.jsonrpc === '2.0' && (msg.result !== undefined || msg.error !== undefined)) {
            if (msg.id === input.id) { resolve(msg); proc.kill(); return; }
          }
        } catch { /* partial */ }
      }
    });
    proc.on('error', reject);
    const req = JSON.stringify(input) + '\n';
    proc.stdin.write(req);
    // Give the server a tick; if no response, reject.
    setTimeout(() => { proc.kill(); reject(new Error('no response: ' + out)); }, 8000);
  });
}

test('GREEN: initialize returns serverInfo + tools capability', async () => {
  const res = await rpc({ jsonrpc: '2.0', id: 1, method: 'initialize', params: {} });
  assert.equal(res.jsonrpc, '2.0');
  assert.equal(res.result.serverInfo.name, 'bebop');
  assert.ok(res.result.capabilities.tools, 'must advertise tools capability');
});

test('GREEN: tools/list returns all 6 Bebop tools', async () => {
  const res = await rpc({ jsonrpc: '2.0', id: 2, method: 'tools/list', params: {} });
  const names = res.result.tools.map((t: any) => t.name);
  for (const n of ['bebop_boot', 'bebop_recall', 'bebop_remember', 'bebop_govern', 'bebop_route', 'bebop_self_maintain']) {
    assert.ok(names.includes(n), `tools/list must include ${n}`);
  }
  assert.equal(names.length, 6);
});

test('GREEN: tools/call bebop_boot certifies the guard OS', async () => {
  const res = await rpc({ jsonrpc: '2.0', id: 3, method: 'tools/call', params: { name: 'bebop_boot', arguments: {} } });
  const payload = JSON.parse(res.result.content[0].text);
  assert.equal(payload.certified, true, 'guard OS must be certified via MCP');
});

test('RED: malformed JSON yields a parse-error response', async () => {
  const proc = spawn(process.execPath, ['bebop.ts', 'mcp'], { cwd: process.cwd(), stdio: ['pipe', 'pipe', 'inherit'] });
  let out = '';
  const got = new Promise<any>((resolve) => {
    proc.stdout.on('data', (d) => { out += d; if (out.includes('parse error')) resolve(JSON.parse(out.trim())); });
  });
  proc.stdin.write('this is not json\n');
  const res = await got;
  assert.equal(res.error.code, -32700);
  assert.match(res.error.message, /parse error/i);
  proc.kill();
});

test('RED: unknown method yields method-not-found', async () => {
  const res = await rpc({ jsonrpc: '2.0', id: 4, method: 'tools/bogus', params: {} });
  assert.equal(res.error.code, -32601);
});
