// Verify: does injecting mcpServers in session/new actually give the backend MCP tools?
const { spawn } = require('child_process');
const backend = process.argv[2] || 'claude';

const backends = {
  claude: { cmd: 'cmd', args: ['/c', 'claude-agent-acp'] },
  codex: { cmd: 'cmd', args: ['/c', 'codex-acp'] },
  gemini: { cmd: 'cmd', args: ['/c', 'gemini', '--acp'] },
};

const cfg = backends[backend];
if (!cfg) { console.error('Unknown:', backend); process.exit(1); }

console.log(`\n=== ${backend}: verify MCP tools available after injection ===\n`);

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
    try {
      const m = JSON.parse(l.trim());
      // Handle both responses and notifications
      if (m.id && pending.has(m.id)) { pending.get(m.id)(m); pending.delete(m.id); }
      else if (m.method) { console.log(`  [notification] ${m.method}: ${JSON.stringify(m.params).substring(0, 200)}`); }
    } catch {}
  }
});
proc.stderr.on('data', d => {
  const s = d.toString().trim();
  if (s && (s.includes('MCP') || s.includes('mcp') || s.includes('tool') || s.includes('mempalace')))
    console.log(`  [stderr] ${s.substring(0, 200)}`);
});

function send(m, p) {
  return new Promise((res, rej) => {
    const id = msgId++;
    proc.stdin.write(JSON.stringify({jsonrpc:'2.0', id, method:m, params:p}) + '\n');
    pending.set(id, res);
    setTimeout(() => { pending.delete(id); rej(new Error('Timeout')); }, 30000);
  });
}

async function run() {
  try {
    // Init
    await send('initialize', { protocolVersion: 1, capabilities: {}, clientInfo: { name: 'test', version: '1.0' } });
    console.log('1. Initialized');

    // Session WITHOUT MCP
    console.log('\n--- Session WITHOUT mcpServers ---');
    const r1 = await send('session/new', { cwd: 'C:\\Users\\Administrator', mcpServers: [] });
    const sid1 = r1.result?.sessionId;
    console.log(`2. Session: ${sid1?.substring(0, 16)}`);

    // Ask about tools
    const q1 = await send('session/prompt', {
      sessionId: sid1,
      prompt: 'List all MCP tools you have access to. If none, say "NO MCP TOOLS". Be very brief.'
    });
    const reply1 = q1.result?.payloads?.[0]?.text || q1.result?.message || JSON.stringify(q1.result).substring(0, 300);
    console.log(`3. Reply: ${reply1.substring(0, 300)}`);

    // Session WITH MCP
    console.log('\n--- Session WITH mcpServers (mempalace) ---');
    const r2 = await send('session/new', {
      cwd: 'C:\\Users\\Administrator',
      mcpServers: [{ name: "mempalace", type: "http", url: "http://127.0.0.1:18793/mcp", headers: [] }]
    });
    const sid2 = r2.result?.sessionId;
    console.log(`4. Session: ${sid2?.substring(0, 16)}`);

    // Wait a bit for MCP to connect
    await new Promise(r => setTimeout(r, 3000));

    // Ask about tools
    const q2 = await send('session/prompt', {
      sessionId: sid2,
      prompt: 'List all MCP tools you have access to, especially any mempalace tools. If none, say "NO MCP TOOLS". Be very brief.'
    });
    const reply2 = q2.result?.payloads?.[0]?.text || q2.result?.message || JSON.stringify(q2.result).substring(0, 500);
    console.log(`5. Reply: ${reply2.substring(0, 500)}`);

  } catch(e) { console.error('Error:', e.message); }
  finally { proc.kill(); process.exit(0); }
}
setTimeout(run, 3000);
