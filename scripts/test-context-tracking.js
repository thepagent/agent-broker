#!/usr/bin/env node
// E2E test: verify context_used/context_size from usage_update notifications
// Tests with claude-agent-acp (fastest ACP backend)

const { spawn } = require('child_process');
const child = spawn('claude-agent-acp', [], { stdio: ['pipe', 'pipe', 'ignore'], shell: true });
let buf = '', id = 0, sid = '';
let usageUpdates = [];

function send(method, params) {
  id++;
  child.stdin.write(JSON.stringify({ jsonrpc: '2.0', id, method, params }) + '\n');
  return id;
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
        sid = msg.result.sessionId;
        console.log('[OK] session created:', sid);
        setTimeout(() => {
          id++;
          child.stdin.write(JSON.stringify({
            jsonrpc: '2.0', id,
            method: 'session/prompt',
            params: { sessionId: sid, prompt: [{ type: 'text', text: 'reply OK' }] }
          }) + '\n');
          console.log('[OK] prompt sent');
        }, 500);
      }

      if (msg.method === 'session/update') {
        const upd = msg.params?.update;
        if (upd?.sessionUpdate === 'usage_update') {
          usageUpdates.push({ used: upd.used, size: upd.size, cost: upd.cost });
          console.log(`[OK] usage_update: used=${upd.used} size=${upd.size} cost=${JSON.stringify(upd.cost)}`);
        }
      }

      // Prompt done
      if (msg.id === 3 && (msg.result !== undefined || msg.error)) {
        setTimeout(() => {
          console.log('\n=== RESULTS ===');
          console.log(`usage_update count: ${usageUpdates.length}`);
          if (usageUpdates.length > 0) {
            const last = usageUpdates[usageUpdates.length - 1];
            const valid = typeof last.used === 'number' && last.used > 0
                       && typeof last.size === 'number' && last.size > 0;
            console.log(`last: used=${last.used} size=${last.size}`);
            console.log(`valid types: used=${typeof last.used} size=${typeof last.size}`);
            if (valid) {
              console.log(`context: ${last.used} / ${last.size} (${((last.used/last.size)*100).toFixed(1)}%)`);
              console.log('\n✅ PASS');
            } else {
              console.log(`\n❌ FAIL — values: used=${last.used} size=${last.size}`);
            }
          } else {
            console.log('\n❌ FAIL — no usage_update received');
          }
          child.kill();
          process.exit(usageUpdates.length > 0 && usageUpdates[0].used > 0 ? 0 : 1);
        }, 500);
      }

      if (msg.method === 'session/request_permission' && msg.id != null) {
        child.stdin.write(JSON.stringify({
          jsonrpc: '2.0', id: msg.id, result: { optionId: 'allow_always' }
        }) + '\n');
      }
    } catch {}
  }
});

send('initialize', { protocolVersion: 1, clientCapabilities: {}, clientInfo: { name: 'test', version: '0.1' } });
setTimeout(() => send('session/new', { cwd: process.cwd(), mcpServers: [] }), 1500);
setTimeout(() => { console.log('\n❌ TIMEOUT'); child.kill(); process.exit(1); }, 60000);
