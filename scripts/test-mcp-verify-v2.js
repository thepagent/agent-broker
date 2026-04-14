// Simpler: just check session/new response and notifications for MCP tool info
const { spawn } = require('child_process');
const backend = process.argv[2] || 'claude';

const backends = {
  claude: { cmd: 'cmd', args: ['/c', 'claude-agent-acp'] },
  codex: { cmd: 'cmd', args: ['/c', 'codex-acp'] },
  gemini: { cmd: 'cmd', args: ['/c', 'gemini', '--acp'] },
};
const cfg = backends[backend];
if (!cfg) { process.exit(1); }

console.log(`\n=== ${backend}: check session/new response for MCP info ===\n`);

const proc = spawn(cfg.cmd, cfg.args, {
  stdio: ['pipe', 'pipe', 'pipe'],
  cwd: 'C:\\Users\\Administrator',
  env: { ...process.env, OPENAB_BOT: '1' }
});

let buffer = '', msgId = 1, pending = new Map(), notifications = [];
proc.stdout.on('data', d => {
  buffer += d.toString();
  const lines = buffer.split('\n'); buffer = lines.pop();
  for (const l of lines) {
    if (!l.trim()) continue;
    try {
      const m = JSON.parse(l.trim());
      if (m.id && pending.has(m.id)) { pending.get(m.id)(m); pending.delete(m.id); }
      else if (m.method) { notifications.push(m); }
    } catch {}
  }
});
proc.stderr.on('data', () => {});

function send(m, p) {
  return new Promise((res, rej) => {
    const id = msgId++;
    proc.stdin.write(JSON.stringify({jsonrpc:'2.0', id, method:m, params:p}) + '\n');
    pending.set(id, res);
    setTimeout(() => { pending.delete(id); rej(new Error('Timeout')); }, 25000);
  });
}

async function run() {
  try {
    await send('initialize', { protocolVersion: 1, capabilities: {}, clientInfo: { name: 'test', version: '1.0' } });

    // Session WITHOUT MCP
    notifications = [];
    console.log('--- WITHOUT mcpServers ---');
    const r1 = await send('session/new', { cwd: 'C:\\Users\\Administrator', mcpServers: [] });
    await new Promise(r => setTimeout(r, 5000)); // wait for notifications
    console.log(`session/new result keys: ${Object.keys(r1.result || {}).join(', ')}`);
    console.log(`Full result: ${JSON.stringify(r1.result).substring(0, 300)}`);
    const toolNotifs1 = notifications.filter(n =>
      JSON.stringify(n).toLowerCase().includes('tool') ||
      JSON.stringify(n).toLowerCase().includes('mcp')
    );
    console.log(`MCP/tool notifications: ${toolNotifs1.length}`);
    toolNotifs1.forEach(n => console.log(`  ${n.method}: ${JSON.stringify(n.params).substring(0, 200)}`));

    // Session WITH MCP
    notifications = [];
    console.log('\n--- WITH mcpServers (mempalace HTTP) ---');
    const r2 = await send('session/new', {
      cwd: 'C:\\Users\\Administrator',
      mcpServers: [{ name: "mempalace", type: "http", url: "http://127.0.0.1:18793/mcp", headers: [] }]
    });
    await new Promise(r => setTimeout(r, 8000)); // longer wait for MCP connection
    console.log(`session/new result keys: ${Object.keys(r2.result || {}).join(', ')}`);
    console.log(`Full result: ${JSON.stringify(r2.result).substring(0, 500)}`);
    const toolNotifs2 = notifications.filter(n =>
      JSON.stringify(n).toLowerCase().includes('tool') ||
      JSON.stringify(n).toLowerCase().includes('mcp') ||
      JSON.stringify(n).toLowerCase().includes('mempalace')
    );
    console.log(`MCP/tool notifications: ${toolNotifs2.length}`);
    toolNotifs2.forEach(n => console.log(`  ${n.method}: ${JSON.stringify(n.params).substring(0, 300)}`));

    // Show ALL notifications for comparison
    console.log(`\nAll notifications (with MCP): ${notifications.length}`);
    notifications.forEach(n => console.log(`  ${n.method}: ${JSON.stringify(n.params).substring(0, 150)}`));

  } catch(e) { console.error('Error:', e.message); }
  finally { proc.kill(); process.exit(0); }
}
setTimeout(run, 3000);
