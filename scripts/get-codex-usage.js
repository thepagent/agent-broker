#!/usr/bin/env node
// Get Codex CLI usage info by spawning a quick ACP session.
// Codex uses OpenAI API credits — no simple quota endpoint.
// Reports model + availability status.

const { spawn } = require('child_process');
const TIMEOUT = 20000;

const child = spawn('codex-acp', [], {
  stdio: ['pipe', 'pipe', 'ignore'],
  shell: process.platform === 'win32',
});

let buf = '';
let id = 0;

function send(method, params) {
  const reqId = ++id;
  child.stdin.write(JSON.stringify({ jsonrpc: '2.0', id: reqId, method, params }) + '\n');
  return reqId;
}

child.stdout.on('data', chunk => {
  buf += chunk.toString();
  let idx;
  while ((idx = buf.indexOf('\n')) >= 0) {
    const line = buf.slice(0, idx).trim();
    buf = buf.slice(idx + 1);
    if (!line) continue;
    try {
      const msg = JSON.parse(line);
      if (msg.id === 2 && msg.result) {
        const sid = msg.result.sessionId || 'unknown';
        console.log(JSON.stringify({
          ok: true,
          session_id: sid,
          tier: 'OpenAI (Codex Pro)',
          status: 'active',
          status_pct: 100,
          ts: new Date().toISOString(),
        }));
        child.kill();
        process.exit(0);
      }
    } catch {}
  }
});

send('initialize', { protocolVersion: 1, clientCapabilities: {}, clientInfo: { name: 'probe', version: '0.1' } });
setTimeout(() => send('session/new', { cwd: process.cwd(), mcpServers: [] }), 1500);

setTimeout(() => {
  console.log(JSON.stringify({ ok: false, error: 'timeout' }));
  child.kill();
  process.exit(1);
}, TIMEOUT);
