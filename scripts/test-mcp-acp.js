// Test ACP mcpServers support for each backend
// Usage: node test-mcp-acp.js <backend>
// Backends: claude, copilot, gemini, codex

const { spawn } = require('child_process');
const readline = require('readline');

const backend = process.argv[2] || 'claude';
const testMcpServers = [
  { name: "test-mempalace", url: "http://127.0.0.1:18793/mcp" }
];

const backends = {
  claude: { cmd: 'cmd', args: ['/c', 'claude-agent-acp'] },
  copilot: { cmd: 'node', args: ['C:\\Users\\Administrator\\openab\\vendor\\copilot-agent-acp\\copilot-agent-acp.js'] },
  gemini: { cmd: 'cmd', args: ['/c', 'gemini', '--acp'] },
  codex: { cmd: 'cmd', args: ['/c', 'codex-acp'] },
};

const cfg = backends[backend];
if (!cfg) { console.error('Unknown backend:', backend); process.exit(1); }

console.log(`\n=== Testing ${backend} ACP mcpServers support ===`);
console.log(`Command: ${cfg.cmd} ${cfg.args.join(' ')}`);

const proc = spawn(cfg.cmd, cfg.args, {
  stdio: ['pipe', 'pipe', 'pipe'],
  cwd: 'C:\\Users\\Administrator',
  env: { ...process.env, OPENAB_BOT: '1' }
});

let buffer = '';
let msgId = 1;
const pending = new Map();
let sessionId = null;

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

proc.stderr.on('data', (d) => {
  const s = d.toString().trim();
  if (s) console.error(`  [stderr] ${s.substring(0, 200)}`);
});

function send(method, params) {
  return new Promise((resolve, reject) => {
    const id = msgId++;
    const msg = { jsonrpc: '2.0', id, method, params };
    pending.set(id, resolve);
    proc.stdin.write(JSON.stringify(msg) + '\n');
    setTimeout(() => {
      if (pending.has(id)) {
        pending.delete(id);
        reject(new Error(`Timeout waiting for ${method}`));
      }
    }, 15000);
  });
}

async function run() {
  try {
    // Step 1: Initialize
    console.log('\n1. Sending initialize...');
    const initResp = await send('initialize', {
      protocolVersion: '2024-11-05',
      capabilities: {},
      clientInfo: { name: 'test', version: '1.0' }
    });
    console.log('   Result:', initResp.error ? `ERROR: ${JSON.stringify(initResp.error)}` : 'OK');

    // Step 2: session/new WITHOUT mcpServers (baseline)
    console.log('\n2. session/new WITHOUT mcpServers (baseline)...');
    const baseResp = await send('session/new', {
      cwd: 'C:\\Users\\Administrator',
      mcpServers: []
    });
    if (baseResp.error) {
      console.log('   ERROR:', JSON.stringify(baseResp.error));
    } else {
      sessionId = baseResp.result?.sessionId;
      console.log('   OK, sessionId:', sessionId?.substring(0, 20) + '...');
    }

    // Step 3: session/new WITH mcpServers
    console.log('\n3. session/new WITH mcpServers...');
    const mcpResp = await send('session/new', {
      cwd: 'C:\\Users\\Administrator',
      mcpServers: testMcpServers
    });
    if (mcpResp.error) {
      console.log('   ERROR:', JSON.stringify(mcpResp.error));
      console.log('   >>> mcpServers NOT supported <<<');
    } else {
      const sid2 = mcpResp.result?.sessionId;
      console.log('   OK, sessionId:', sid2?.substring(0, 20) + '...');

      // Check if MCP tools are available
      const models = mcpResp.result?.models;
      const tools = mcpResp.result?.tools;
      console.log('   Models:', models?.currentModelId || 'N/A');
      console.log('   Tools in response:', tools ? JSON.stringify(tools).substring(0, 100) : 'N/A');
      console.log('   >>> mcpServers ACCEPTED <<<');
    }

    console.log('\n=== DONE ===\n');
  } catch (e) {
    console.error('Error:', e.message);
  } finally {
    proc.kill();
    process.exit(0);
  }
}

// Give process time to start
setTimeout(run, 2000);
