const { spawn } = require('child_process');
console.log('=== Codex: array-named-http only ===');
const proc = spawn('cmd', ['/c', 'codex-acp'], {
  stdio: ['pipe', 'pipe', 'pipe'],
  cwd: 'C:\\Users\\Administrator',
  env: { ...process.env, OPENAB_BOT: '1' }
});
let buffer = '', msgId = 1, pending = new Map();
proc.stdout.on('data', d => {
  buffer += d.toString();
  const lines = buffer.split('\n'); buffer = lines.pop();
  for (const l of lines) { try { const m = JSON.parse(l.trim()); if (m.id && pending.has(m.id)) { pending.get(m.id)(m); pending.delete(m.id); } } catch {} }
});
proc.stderr.on('data', () => {});
function send(m, p) { return new Promise((res, rej) => { const id = msgId++; proc.stdin.write(JSON.stringify({jsonrpc:'2.0',id,method:m,params:p})+'\n'); pending.set(id, res); setTimeout(() => { pending.delete(id); rej(new Error('Timeout')); }, 20000); }); }
async function run() {
  try {
    await send('initialize', { protocolVersion: 1, capabilities: {}, clientInfo: { name: 'test', version: '1.0' } });
    console.log('init OK');
    const r = await send('session/new', { cwd: 'C:\\Users\\Administrator', mcpServers: [{ name: "mempalace", type: "http", url: "http://127.0.0.1:18793/mcp", headers: [] }] });
    console.log(r.error ? `FAIL: ${JSON.stringify(r.error)}` : `OK ✅ sid=${r.result?.sessionId?.substring(0,16)}`);
  } catch(e) { console.log('Error:', e.message); }
  finally { proc.kill(); process.exit(0); }
}
setTimeout(run, 3000);
