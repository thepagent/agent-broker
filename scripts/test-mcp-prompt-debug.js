// Debug: dump raw session/prompt response
const { spawn } = require('child_process');
const backend = process.argv[2] || 'codex';
const backends = {
  codex: { cmd: 'cmd', args: ['/c', 'codex-acp'] },
  gemini: { cmd: 'cmd', args: ['/c', 'gemini', '--acp'] },
};
const cfg = backends[backend];
console.log(`\n=== ${backend}: raw prompt response ===\n`);

const proc = spawn(cfg.cmd, cfg.args, {
  stdio: ['pipe', 'pipe', 'pipe'],
  cwd: 'C:\\Users\\Administrator',
  env: { ...process.env, OPENAB_BOT: '1' }
});

let buffer = '', msgId = 1, pending = new Map();
// Dump ALL stdout
proc.stdout.on('data', d => {
  const raw = d.toString();
  buffer += raw;
  const lines = buffer.split('\n'); buffer = lines.pop();
  for (const l of lines) {
    if (!l.trim()) continue;
    try {
      const m = JSON.parse(l.trim());
      if (m.id && pending.has(m.id)) { pending.get(m.id)(m); pending.delete(m.id); }
      // Log everything after session/prompt
      if (msgId > 3) console.log(`[stdout] ${l.substring(0, 400)}`);
    } catch {
      if (msgId > 3) console.log(`[raw] ${l.substring(0, 400)}`);
    }
  }
});
proc.stderr.on('data', d => {
  const s = d.toString().trim();
  if (s && msgId > 3) console.log(`[stderr] ${s.substring(0, 200)}`);
});

function send(m, p) {
  return new Promise((res, rej) => {
    const id = msgId++;
    proc.stdin.write(JSON.stringify({jsonrpc:'2.0', id, method:m, params:p}) + '\n');
    pending.set(id, res);
    setTimeout(() => { pending.delete(id); rej(new Error('Timeout')); }, 60000);
  });
}

async function run() {
  try {
    await send('initialize', { protocolVersion: 1, capabilities: {}, clientInfo: { name: 'test', version: '1.0' } });
    const r = await send('session/new', {
      cwd: 'C:\\Users\\Administrator',
      mcpServers: [{ name: "mempalace", type: "http", url: "http://127.0.0.1:18793/mcp", headers: [] }]
    });
    console.log(`Session: ${r.result?.sessionId?.substring(0, 16)}`);
    await new Promise(r => setTimeout(r, 5000));
    console.log('\nSending prompt...\n');
    const q = await send('session/prompt', {
      sessionId: r.result?.sessionId,
      prompt: 'Say "hello" and list any MCP tools you have. Keep it short.'
    });
    console.log(`\n[response id=${q.id}] ${JSON.stringify(q).substring(0, 1000)}`);
  } catch(e) { console.error('Error:', e.message); }
  finally { setTimeout(() => { proc.kill(); process.exit(0); }, 2000); }
}
setTimeout(run, 3000);
