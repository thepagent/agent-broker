// Final E2E test: correct prompt format for all backends
const { spawn } = require('child_process');
const fs = require('fs');
const path = require('path');

const backend = process.argv[2] || 'claude';
const profileDir = `C:\\Users\\Administrator\\openab\\data\\mcp-profiles\\${backend === 'claude' ? 'cicx' : backend === 'copilot' ? 'gitx' : backend === 'gemini' ? 'giminix' : 'codex'}`;
const userId = '844236700611379200';

const backends = {
  claude: { cmd: 'cmd', args: ['/c', 'claude-agent-acp'] },
  codex: { cmd: 'cmd', args: ['/c', 'codex-acp'] },
  gemini: { cmd: 'cmd', args: ['/c', 'gemini', '--acp'] },
};
const cfg = backends[backend];
if (!cfg) { process.exit(1); }

// Read profile
let mcpServers = [];
try {
  const profile = JSON.parse(fs.readFileSync(path.join(profileDir, `${userId}.json`), 'utf8'));
  mcpServers = Object.entries(profile.mcpServers || {}).map(([name, config]) => ({ name, ...config }));
} catch {}

console.log(`\n=== E2E: ${backend} | ${mcpServers.length} MCP server(s) ===\n`);

const proc = spawn(cfg.cmd, cfg.args, {
  stdio: ['pipe', 'pipe', 'pipe'],
  cwd: 'C:\\Users\\Administrator',
  env: { ...process.env, OPENAB_BOT: '1' }
});

let buffer = '', msgId = 1, pending = new Map(), allOutput = [];
proc.stdout.on('data', d => {
  buffer += d.toString();
  const lines = buffer.split('\n'); buffer = lines.pop();
  for (const l of lines) {
    if (!l.trim()) continue;
    try {
      const m = JSON.parse(l.trim());
      if (m.id && pending.has(m.id)) { pending.get(m.id)(m); pending.delete(m.id); }
      else { allOutput.push(m); }
    } catch {}
  }
});
proc.stderr.on('data', () => {});

function send(m, p) {
  return new Promise((res, rej) => {
    const id = msgId++;
    proc.stdin.write(JSON.stringify({jsonrpc:'2.0', id, method:m, params:p}) + '\n');
    pending.set(id, res);
    setTimeout(() => { pending.delete(id); rej(new Error('Timeout')); }, 90000);
  });
}

async function run() {
  try {
    await send('initialize', { protocolVersion: 1, capabilities: {}, clientInfo: { name: 'e2e', version: '1.0' } });
    console.log('1. Init OK');

    const r = await send('session/new', { cwd: 'C:\\Users\\Administrator', mcpServers });
    if (r.error) { console.log(`2. FAIL: ${r.error.message}`); return; }
    const sid = r.result?.sessionId;
    console.log(`2. Session: ${sid?.substring(0,16)}`);

    await new Promise(r => setTimeout(r, 8000));
    console.log('3. Waited 8s for MCP');

    // Correct format: array of content blocks with type:"text"
    allOutput = [];
    console.log('4. Prompting: "search mempalace for GPU"...');
    const q = await send('session/prompt', {
      sessionId: sid,
      prompt: [{ type: "text", text: "Use the mempalace_search tool to search for 'GPU'. Show the first result briefly." }]
    });

    // Collect streaming notifications for a few seconds
    await new Promise(r => setTimeout(r, 5000));

    // Analyze response
    if (q.error) {
      console.log(`5. Error: ${JSON.stringify(q.error).substring(0, 200)}`);
    } else {
      console.log(`5. Prompt accepted (response id=${q.id})`);
    }

    // Check all output for MCP tool usage
    const allText = JSON.stringify(allOutput);
    const hasTool = allText.includes('mempalace') || allText.includes('tool_use') || allText.includes('mcp_tool');
    const hasGpu = allText.toLowerCase().includes('gpu') || allText.toLowerCase().includes('vram');

    console.log(`\n6. Analysis:`);
    console.log(`   Notifications received: ${allOutput.length}`);
    console.log(`   Contains 'mempalace': ${allText.includes('mempalace')}`);
    console.log(`   Contains tool_use: ${allText.includes('tool_use')}`);
    console.log(`   Contains GPU/VRAM: ${hasGpu}`);

    // Show relevant notifications
    for (const n of allOutput) {
      const s = JSON.stringify(n);
      if (s.includes('mempalace') || s.includes('tool') || s.includes('GPU') || s.includes('gpu')) {
        console.log(`   [relevant] ${s.substring(0, 300)}`);
      }
    }

    if (hasTool && hasGpu) {
      console.log('\n>>> MCP BRIDGE FULLY WORKING ✅ <<<');
    } else if (hasTool) {
      console.log('\n>>> MCP TOOL DETECTED ✅ (no GPU result — MCP server may need time) <<<');
    } else {
      console.log('\n>>> MCP tool not detected in output — check manually <<<');
      // Dump last few notifications for debug
      console.log('\nLast 3 notifications:');
      allOutput.slice(-3).forEach(n => console.log(JSON.stringify(n).substring(0, 300)));
    }

  } catch(e) { console.error('Error:', e.message); }
  finally { proc.kill(); process.exit(0); }
}
setTimeout(run, 3000);
