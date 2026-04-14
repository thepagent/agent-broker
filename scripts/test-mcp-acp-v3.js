// Debug: show full error details for Claude and Codex
const { spawn } = require('child_process');
const backend = process.argv[2] || 'claude';

const backends = {
  claude: { cmd: 'cmd', args: ['/c', 'claude-agent-acp'] },
  gemini: { cmd: 'cmd', args: ['/c', 'gemini', '--acp'] },
  codex: { cmd: 'cmd', args: ['/c', 'codex-acp'] },
};

const cfg = backends[backend];
if (!cfg) { console.error('Unknown backend:', backend); process.exit(1); }

// Different mcpServers structures to try
const tests = [
  { name: 'array-of-http', mcpServers: [{ type: "http", url: "http://127.0.0.1:18793/mcp", headers: [] }] },
  { name: 'object-map', mcpServers: { "mempalace": { type: "http", url: "http://127.0.0.1:18793/mcp", headers: [] } } },
  { name: 'object-cmd', mcpServers: { "mempalace": { command: "echo", args: ["test"], env: [] } } },
  { name: 'array-named-http', mcpServers: [{ name: "mempalace", type: "http", url: "http://127.0.0.1:18793/mcp", headers: [] }] },
  { name: 'array-named-cmd', mcpServers: [{ name: "mempalace", command: "echo", args: ["test"], env: [] }] },
  { name: 'no-mcp-field', _skip_mcp: true },
];

console.log(`\n=== ${backend} — debug mcpServers structures ===\n`);

const proc = spawn(cfg.cmd, cfg.args, {
  stdio: ['pipe', 'pipe', 'pipe'],
  cwd: 'C:\\Users\\Administrator',
  env: { ...process.env, OPENAB_BOT: '1' }
});

let buffer = '';
let msgId = 1;
const pending = new Map();

proc.stdout.on('data', (data) => {
  buffer += data.toString();
  const lines = buffer.split('\n');
  buffer = lines.pop();
  for (const line of lines) {
    if (!line.trim()) continue;
    try {
      const msg = JSON.parse(line.trim());
      if (msg.id && pending.has(msg.id)) {
        pending.get(msg.id)(msg);
        pending.delete(msg.id);
      }
    } catch {}
  }
});

proc.stderr.on('data', () => {});

function send(method, params) {
  return new Promise((resolve, reject) => {
    const id = msgId++;
    proc.stdin.write(JSON.stringify({ jsonrpc: '2.0', id, method, params }) + '\n');
    pending.set(id, resolve);
    setTimeout(() => { pending.delete(id); reject(new Error('Timeout')); }, 15000);
  });
}

async function run() {
  try {
    await send('initialize', { protocolVersion: 1, capabilities: {}, clientInfo: { name: 'test', version: '1.0' } }).catch(() => {});

    for (const t of tests) {
      const params = { cwd: 'C:\\Users\\Administrator' };
      if (!t._skip_mcp) params.mcpServers = t.mcpServers;

      try {
        const resp = await send('session/new', params);
        if (resp.error) {
          console.log(`${t.name}:`);
          console.log(`  FAIL: ${JSON.stringify(resp.error).substring(0, 300)}`);
        } else {
          console.log(`${t.name}:`);
          console.log(`  OK ✅ sid=${resp.result?.sessionId?.substring(0, 16)}`);
        }
      } catch (e) {
        console.log(`${t.name}: TIMEOUT`);
      }
      console.log('');
    }
  } catch (e) {
    console.error('Fatal:', e.message);
  } finally {
    proc.kill();
    process.exit(0);
  }
}

setTimeout(run, 2000);
