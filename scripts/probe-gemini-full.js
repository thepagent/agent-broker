#!/usr/bin/env node
// Probe Gemini ACP for ALL available data: session/new response, _meta methods, etc.
const { spawn } = require('child_process');
const child = spawn('gemini', ['--acp'], { stdio: ['pipe', 'pipe', 'ignore'], shell: true });
let buf = '';
let id = 0;
const responses = {};

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
      if (msg.id) responses[msg.id] = msg;
      // Also capture notifications
      if (msg.method) {
        console.error('[NOTIFICATION]', msg.method, JSON.stringify(msg.params).slice(0, 500));
      }
    } catch {}
  }
});

(async () => {
  send('initialize', { protocolVersion: 1, clientCapabilities: {}, clientInfo: { name: 'probe', version: '0.1' } });
  await new Promise(r => setTimeout(r, 3000));

  const sessId = send('session/new', { cwd: process.cwd(), mcpServers: [] });
  await new Promise(r => setTimeout(r, 5000));

  // Try various _meta methods
  const metaMethods = ['_meta/getUsage', '_meta/getQuota', '_meta/getStatus', '_meta/ping',
    'session/getUsage', 'session/quota', 'session/status'];
  const sessionId = responses[sessId]?.result?.sessionId;
  for (const m of metaMethods) {
    send(m, sessionId ? { sessionId } : {});
  }
  await new Promise(r => setTimeout(r, 3000));

  // Dump everything
  console.log(JSON.stringify({
    session_new: responses[sessId]?.result,
    meta_responses: Object.fromEntries(
      Object.entries(responses)
        .filter(([k]) => parseInt(k) > sessId)
        .map(([k, v]) => [metaMethods[parseInt(k) - sessId - 1] || k, v.result || v.error])
    ),
  }, null, 2));

  child.kill();
  process.exit(0);
})();
