// End-to-end test: simulate what OpenAB does after Phase 2
// 1. Read profile JSON (like read_mcp_profile in config.rs)
// 2. Build mcpServers array (like mcp_servers_for_user in discord.rs)
// 3. Pass to session/new (like connection.rs)
// 4. Send a prompt asking about MCP tools
// 5. Check response for mempalace tools

const { spawn } = require('child_process');
const fs = require('fs');
const path = require('path');

const backend = process.argv[2] || 'claude';
const profileDir = process.argv[3] || 'C:\\Users\\Administrator\\openab\\data\\mcp-profiles\\cicx';
const userId = '844236700611379200';

const backends = {
  claude: { cmd: 'cmd', args: ['/c', 'claude-agent-acp'] },
  codex: { cmd: 'cmd', args: ['/c', 'codex-acp'] },
  gemini: { cmd: 'cmd', args: ['/c', 'gemini', '--acp'] },
};
const cfg = backends[backend];
if (!cfg) { console.error('Unknown:', backend); process.exit(1); }

// Step 1: Read profile (simulates read_mcp_profile)
console.log(`\n=== E2E MCP Bridge Test: ${backend} ===\n`);
const profilePath = path.join(profileDir, `${userId}.json`);
let mcpServers = [];
try {
  const profile = JSON.parse(fs.readFileSync(profilePath, 'utf8'));
  const servers = profile.mcpServers || {};
  mcpServers = Object.entries(servers).map(([name, config]) => ({
    name,
    ...config
  }));
  console.log(`1. Profile loaded: ${mcpServers.length} MCP server(s)`);
  mcpServers.forEach(s => console.log(`   - ${s.name}: ${s.type} ${s.url || s.command}`));
} catch (e) {
  console.log(`1. No profile found: ${e.message}`);
}

// Step 2: Start ACP backend
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
      if (m.id && pending.has(m.id)) { pending.get(m.id)(m); pending.delete(m.id); }
      // Log MCP-related notifications
      if (m.method && JSON.stringify(m).toLowerCase().includes('mcp')) {
        console.log(`   [notification] ${m.method}: ${JSON.stringify(m.params).substring(0, 200)}`);
      }
    } catch {}
  }
});
proc.stderr.on('data', d => {
  const s = d.toString().trim();
  if (s && (s.toLowerCase().includes('mcp') || s.toLowerCase().includes('mempalace') || s.toLowerCase().includes('tool')))
    console.log(`   [stderr] ${s.substring(0, 200)}`);
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
    // Initialize
    await send('initialize', { protocolVersion: 1, capabilities: {}, clientInfo: { name: 'e2e-test', version: '1.0' } });
    console.log('2. ACP initialized');

    // Step 3: session/new WITH mcpServers from profile
    console.log(`3. Creating session with ${mcpServers.length} MCP server(s)...`);
    const r = await send('session/new', {
      cwd: 'C:\\Users\\Administrator',
      mcpServers: mcpServers
    });
    if (r.error) {
      console.log(`   FAIL: ${JSON.stringify(r.error).substring(0, 200)}`);
      return;
    }
    const sid = r.result?.sessionId;
    console.log(`   OK: session ${sid?.substring(0, 16)}`);

    // Wait for MCP to connect asynchronously
    console.log('4. Waiting 8s for MCP connection...');
    await new Promise(r => setTimeout(r, 8000));

    // Step 4: Send prompt asking about tools
    console.log('5. Sending prompt to verify MCP tools...');

    // Build prompt in the format each backend expects
    const promptPayload = backend === 'codex'
      ? { sessionId: sid, prompt: [{ type: "text", text: "Search mempalace for 'GPU'. Use the mempalace_search tool. Be brief." }] }
      : { sessionId: sid, prompt: [{ type: "human", text: "Search mempalace for 'GPU'. Use the mempalace_search tool. Reply briefly with what you found." }] };

    const q = await send('session/prompt', promptPayload);

    if (q.error) {
      console.log(`   Prompt error: ${JSON.stringify(q.error).substring(0, 300)}`);
    } else {
      // Try to find text in various response formats
      const result = q.result || {};
      const payloads = result.payloads || [];
      let foundMcp = false;
      let replyText = '';

      for (const p of payloads) {
        if (p.text) replyText += p.text + '\n';
        if (p.type === 'tool_use' || p.type === 'tool_result') {
          foundMcp = true;
          console.log(`   [tool] ${p.name || p.type}: ${JSON.stringify(p).substring(0, 200)}`);
        }
      }

      if (!replyText && result.content) {
        for (const c of (Array.isArray(result.content) ? result.content : [result.content])) {
          if (c.text) replyText += c.text + '\n';
          if (c.type === 'tool_use') { foundMcp = true; console.log(`   [tool] ${c.name}`); }
        }
      }

      if (!replyText) replyText = JSON.stringify(result).substring(0, 500);

      console.log(`\n6. Reply (first 300 chars):\n   ${replyText.substring(0, 300)}`);
      console.log(`\n7. MCP tool usage detected: ${foundMcp ? '✅ YES' : '❓ check reply text'}`);

      // Check if reply mentions mempalace
      if (replyText.toLowerCase().includes('mempalace') || replyText.toLowerCase().includes('gpu') || replyText.toLowerCase().includes('drawer')) {
        console.log('   >>> MCP BRIDGE WORKING ✅ <<<');
      } else {
        console.log('   >>> Reply does not mention mempalace/GPU — may need manual verification <<<');
      }
    }

  } catch(e) { console.error('Error:', e.message); }
  finally { proc.kill(); process.exit(0); }
}

setTimeout(run, 3000);
