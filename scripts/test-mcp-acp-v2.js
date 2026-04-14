// Test ACP mcpServers with correct format per backend
const { spawn } = require('child_process');
const backend = process.argv[2] || 'claude';

// Format variants to test
const formats = {
  http: { type: "http", url: "http://127.0.0.1:18793/mcp", headers: [] },
  sse: { type: "sse", url: "http://127.0.0.1:18793/mcp", headers: [] },
  stdio: { command: "wsl.exe", args: ["-u", "root", "-d", "Ubuntu", "--", "/root/.local/share/pipx/venvs/mempalace/bin/python3", "-m", "mempalace.mcp_server", "--palace", "/root/.mempalace/palace"], env: [] },
  url_only: { url: "http://127.0.0.1:18793/mcp" },
  codex_http: { type: "http", url: "http://127.0.0.1:18793/mcp" },
};

const backends = {
  claude: { cmd: 'cmd', args: ['/c', 'claude-agent-acp'] },
  copilot: { cmd: 'node', args: ['C:\\Users\\Administrator\\openab\\vendor\\copilot-agent-acp\\copilot-agent-acp.js'] },
  gemini: { cmd: 'cmd', args: ['/c', 'gemini', '--acp'] },
  codex: { cmd: 'cmd', args: ['/c', 'codex-acp'] },
};

const cfg = backends[backend];
if (!cfg) { console.error('Unknown backend:', backend); process.exit(1); }

console.log(`\n=== Testing ${backend} — all mcpServers formats ===\n`);

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

proc.stderr.on('data', () => {}); // suppress

function send(method, params) {
  return new Promise((resolve, reject) => {
    const id = msgId++;
    proc.stdin.write(JSON.stringify({ jsonrpc: '2.0', id, method, params }) + '\n');
    pending.set(id, resolve);
    setTimeout(() => { pending.delete(id); reject(new Error(`Timeout: ${method}`)); }, 15000);
  });
}

async function run() {
  try {
    // Initialize (use number for claude/gemini compatibility)
    await send('initialize', {
      protocolVersion: 1,
      capabilities: {},
      clientInfo: { name: 'test', version: '1.0' }
    }).catch(() => {});

    // Baseline
    const base = await send('session/new', { cwd: 'C:\\Users\\Administrator', mcpServers: [] });
    console.log(`baseline (empty): ${base.error ? 'FAIL ' + base.error.message : 'OK sid=' + base.result?.sessionId?.substring(0,12)}`);

    // Test each format
    for (const [name, fmt] of Object.entries(formats)) {
      try {
        const resp = await send('session/new', {
          cwd: 'C:\\Users\\Administrator',
          mcpServers: [fmt]
        });
        if (resp.error) {
          console.log(`${name.padEnd(12)}: FAIL — ${resp.error.message?.substring(0, 80)}`);
        } else {
          console.log(`${name.padEnd(12)}: OK ✅  sid=${resp.result?.sessionId?.substring(0,12)}`);
        }
      } catch (e) {
        console.log(`${name.padEnd(12)}: TIMEOUT`);
      }
    }
  } catch (e) {
    console.error('Fatal:', e.message);
  } finally {
    proc.kill();
    process.exit(0);
  }
}

setTimeout(run, 2000);
