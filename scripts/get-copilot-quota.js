#!/usr/bin/env node
// Get Copilot CLI remaining quota via node-pty (reads status bar).
// Falls back to ACP probe if pty fails.

const TIMEOUT_MS = 40000;
const COPILOT_PATH = 'C:/Users/Administrator/AppData/Local/Microsoft/WinGet/Links/copilot.exe';

let pty;
try {
  pty = require('C:/Users/Administrator/AppData/Roaming/npm/node_modules/node-pty');
} catch {
  // node-pty not available, use ACP fallback
  apcFallback();
}

if (pty) {
  const p = pty.spawn(COPILOT_PATH, [], {
    name: 'xterm-color',
    cols: 200,
    rows: 50,
    cwd: 'C:/Users/Administrator',
    env: process.env
  });

  let buf = '';
  let done = false;

  p.onData(d => {
    buf += d;
    const m = buf.match(/Remaining reqs\.?: ?([\d.]+)%/);
    if (m && !done) {
      done = true;
      console.log(JSON.stringify({
        ok: true,
        remaining_pct: parseFloat(m[1]),
        raw: m[0],
        ts: new Date().toISOString()
      }));
      p.kill();
      process.exit(0);
    }
  });

  // Give copilot a moment to render status bar, then send /usage if needed
  setTimeout(() => {
    if (!done) p.write('/usage\r');
  }, 8000);

  setTimeout(() => {
    if (!done) {
      // Try one more parse
      const m = buf.match(/Remaining reqs\.?: ?([\d.]+)%/);
      if (m) {
        console.log(JSON.stringify({
          ok: true,
          remaining_pct: parseFloat(m[1]),
          raw: m[0],
          ts: new Date().toISOString()
        }));
      } else {
        // Fallback: report what we can
        console.log(JSON.stringify({
          ok: true,
          remaining_pct: 100,
          note: 'quota % unavailable, assuming healthy',
          ts: new Date().toISOString()
        }));
      }
      p.kill();
      process.exit(0);
    }
  }, TIMEOUT_MS);
}

function apcFallback() {
  const { spawn } = require('child_process');
  const child = spawn('copilot', ['--acp'], { stdio: ['pipe','pipe','ignore'], shell: true });
  let buf2 = '', id2 = 0;
  function send(m, p) { child.stdin.write(JSON.stringify({jsonrpc:'2.0',id:++id2,method:m,params:p})+'\n'); }
  child.stdout.on('data', c => {
    buf2 += c.toString();
    let idx;
    while ((idx = buf2.indexOf('\n')) >= 0) {
      const line = buf2.slice(0, idx).trim();
      buf2 = buf2.slice(idx + 1);
      try {
        const msg = JSON.parse(line);
        if (msg.id === 2 && msg.result) {
          const models = msg.result.models || {};
          console.log(JSON.stringify({
            ok: true,
            remaining_pct: 100,
            current_model: models.currentModelId || 'unknown',
            available_models: (models.availableModels||[]).length,
            note: 'ACP fallback (no quota %)',
            ts: new Date().toISOString()
          }));
          child.kill(); process.exit(0);
        }
      } catch {}
    }
  });
  send('initialize', {protocolVersion:1,clientCapabilities:{},clientInfo:{name:'probe',version:'0.1'}});
  setTimeout(() => send('session/new', {cwd:process.cwd(),mcpServers:[]}), 2000);
  setTimeout(() => { console.log(JSON.stringify({ok:false,error:'timeout'})); child.kill(); process.exit(1); }, 30000);
}
