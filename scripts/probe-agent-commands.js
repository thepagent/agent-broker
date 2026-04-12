#!/usr/bin/env node
// Probe an ACP agent: initialize → session/new → collect available_commands_update
// Usage: node probe-agent-commands.js <name> <command> [args...]
const { spawn } = require('child_process');
const [,, name, cmd, ...args] = process.argv;
if (!name || !cmd) {
  console.error('usage: probe-agent-commands.js <name> <command> [args...]');
  process.exit(2);
}
const child = spawn(cmd, args, { stdio: ['pipe','pipe','inherit'], shell: process.platform === 'win32' });
let buf = '';
let id = 0;
const pending = new Map();
const commands = [];
let sessionId = null;

function send(method, params) {
  const reqId = ++id;
  const req = { jsonrpc: '2.0', id: reqId, method, params };
  child.stdin.write(JSON.stringify(req) + '\n');
  return new Promise((res, rej) => {
    pending.set(reqId, { res, rej });
    setTimeout(() => { if (pending.has(reqId)) { pending.delete(reqId); rej(new Error('timeout '+method)); } }, 60000);
  });
}

child.stdout.on('data', chunk => {
  buf += chunk.toString();
  let idx;
  while ((idx = buf.indexOf('\n')) >= 0) {
    const line = buf.slice(0, idx).trim();
    buf = buf.slice(idx + 1);
    if (!line) continue;
    let msg;
    try { msg = JSON.parse(line); } catch { continue; }
    if (msg.id != null && pending.has(msg.id)) {
      const { res, rej } = pending.get(msg.id);
      pending.delete(msg.id);
      if (msg.error) rej(new Error(JSON.stringify(msg.error)));
      else res(msg.result);
    } else if (msg.method === 'session/update') {
      const upd = msg.params?.update;
      if (upd?.sessionUpdate === 'available_commands_update' && Array.isArray(upd.availableCommands)) {
        commands.push(...upd.availableCommands);
      }
    } else if (msg.method === 'session/request_permission' && msg.id != null) {
      // auto-allow so we don't block
      child.stdin.write(JSON.stringify({jsonrpc:'2.0',id:msg.id,result:{optionId:'allow_always'}})+'\n');
    }
  }
});

(async () => {
  try {
    await send('initialize', { protocolVersion: 1, clientCapabilities: {}, clientInfo: { name: 'probe', version: '0.1' } });
    const s = await send('session/new', { cwd: process.cwd(), mcpServers: [] });
    sessionId = s.sessionId;
    // wait 2s for async availableCommandsUpdate notifications
    await new Promise(r => setTimeout(r, 2000));
  } catch (e) {
    console.error(`[${name}] error:`, e.message);
  }
  const uniq = {};
  for (const c of commands) uniq[c.name] = c.description || '';
  console.log(JSON.stringify({ agent: name, sessionId, count: Object.keys(uniq).length, commands: uniq }, null, 2));
  child.kill();
  process.exit(0);
})();
