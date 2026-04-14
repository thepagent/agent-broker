#!/usr/bin/env node
// Test which ACP backends emit usage_update notifications
// Tests: claude-agent-acp, gemini --acp, codex-acp

const { spawn } = require('child_process');

const backends = [
  { name: 'Claude', cmd: 'claude-agent-acp', args: [], timeout: 45000 },
  { name: 'Gemini', cmd: 'gemini', args: ['--acp'], timeout: 45000 },
  { name: 'Codex', cmd: 'codex-acp', args: [], timeout: 45000 },
];

async function testBackend(backend) {
  return new Promise((resolve) => {
    const child = spawn(backend.cmd, backend.args, { stdio: ['pipe', 'pipe', 'ignore'], shell: true });
    let buf = '', id = 0, sid = '';
    let usageUpdates = [];
    let promptSent = false;

    const send = (m, p) => { id++; child.stdin.write(JSON.stringify({ jsonrpc: '2.0', id, method: m, params: p }) + '\n'); };

    child.stdout.on('data', chunk => {
      buf += chunk.toString();
      let idx;
      while ((idx = buf.indexOf('\n')) >= 0) {
        const line = buf.slice(0, idx).trim(); buf = buf.slice(idx + 1);
        if (!line) continue;
        try {
          const msg = JSON.parse(line);
          if (msg.id === 2 && msg.result) {
            sid = msg.result.sessionId;
            // Set permissive mode first
            const modes = msg.result.modes?.availableModes || [];
            const yolo = modes.find(m => ['yolo','full-access','auto'].includes(m.id));
            if (yolo) {
              id++; child.stdin.write(JSON.stringify({
                jsonrpc:'2.0', id, method:'session/set_mode',
                params: { sessionId: sid, modeId: yolo.id }
              }) + '\n');
            }
            setTimeout(() => {
              if (!promptSent) {
                promptSent = true;
                id++; child.stdin.write(JSON.stringify({
                  jsonrpc:'2.0', id, method:'session/prompt',
                  params: { sessionId: sid, prompt: [{ type:'text', text:'reply OK' }] }
                }) + '\n');
              }
            }, 1000);
          }
          if (msg.method === 'session/update') {
            const upd = msg.params?.update;
            if (upd?.sessionUpdate === 'usage_update') {
              usageUpdates.push({ used: upd.used, size: upd.size });
            }
          }
          // Prompt done
          if (promptSent && msg.id && msg.id > 2 && msg.result !== undefined) {
            setTimeout(() => {
              child.kill();
              resolve({ name: backend.name, updates: usageUpdates.length, last: usageUpdates[usageUpdates.length - 1] || null });
            }, 500);
          }
          if (msg.method === 'session/request_permission' && msg.id != null) {
            child.stdin.write(JSON.stringify({ jsonrpc:'2.0', id: msg.id, result: { optionId: 'allow_always' } }) + '\n');
          }
        } catch {}
      }
    });

    send('initialize', { protocolVersion: 1, clientCapabilities: {}, clientInfo: { name: 'test', version: '0.1' } });
    setTimeout(() => send('session/new', { cwd: process.cwd(), mcpServers: [] }), 2000);
    setTimeout(() => { child.kill(); resolve({ name: backend.name, updates: 0, last: null, timeout: true }); }, backend.timeout);
  });
}

(async () => {
  for (const b of backends) {
    process.stdout.write(`Testing ${b.name}... `);
    const result = await testBackend(b);
    if (result.timeout) {
      console.log('⏱️  TIMEOUT (prompt too slow)');
    } else if (result.updates > 0 && result.last?.used > 0) {
      console.log(`✅ ${result.updates} updates — used=${result.last.used} size=${result.last.size} (${((result.last.used/result.last.size)*100).toFixed(1)}%)`);
    } else if (result.updates > 0) {
      console.log(`⚠️  ${result.updates} updates but used=0 — ${JSON.stringify(result.last)}`);
    } else {
      console.log('❌ No usage_update received');
    }
  }
})();
