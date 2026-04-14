// Send a prompt asking backend to list MCP tools — compare with/without mcpServers
const { spawn } = require('child_process');
const backend = process.argv[2] || 'codex';

const backends = {
  codex: { cmd: 'cmd', args: ['/c', 'codex-acp'] },
  gemini: { cmd: 'cmd', args: ['/c', 'gemini', '--acp'] },
};
const cfg = backends[backend];
if (!cfg) { process.exit(1); }

console.log(`\n=== ${backend}: prompt test for MCP tools ===\n`);

const proc = spawn(cfg.cmd, cfg.args, {
  stdio: ['pipe', 'pipe', 'pipe'],
  cwd: 'C:\\Users\\Administrator',
  env: { ...process.env, OPENAB_BOT: '1' }
});

let buffer = '', msgId = 1, pending = new Map();
proc.stdout.on('data', d => {
  buffer += d.toString();
  const lines = buffer.split('\n'); buffer = lines.pop();
  for (const l of lines) {
    if (!l.trim()) continue;
    try { const m = JSON.parse(l.trim()); if (m.id && pending.has(m.id)) { pending.get(m.id)(m); pending.delete(m.id); } } catch {}
  }
});
proc.stderr.on('data', () => {});

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

    // WITH MCP
    console.log('Creating session WITH mempalace MCP...');
    const r = await send('session/new', {
      cwd: 'C:\\Users\\Administrator',
      mcpServers: [{ name: "mempalace", type: "http", url: "http://127.0.0.1:18793/mcp", headers: [] }]
    });
    const sid = r.result?.sessionId;
    console.log(`Session: ${sid?.substring(0, 16)}`);

    // Wait for MCP to connect
    await new Promise(r => setTimeout(r, 5000));

    console.log('\nSending prompt: "Do you have mempalace MCP tools? List them briefly."');
    const q = await send('session/prompt', {
      sessionId: sid,
      prompt: 'Do you have any mempalace MCP tools available? List the tool names briefly. Answer in under 100 words.'
    });

    // Extract reply from various response formats
    const result = q.result || {};
    const text = result.payloads?.[0]?.text
      || result.message
      || result.content?.[0]?.text
      || result.reply
      || JSON.stringify(result).substring(0, 500);
    console.log(`\nReply:\n${text.substring(0, 500)}`);

  } catch(e) { console.error('Error:', e.message); }
  finally { proc.kill(); process.exit(0); }
}
setTimeout(run, 3000);
