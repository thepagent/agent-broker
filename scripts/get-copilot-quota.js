#!/usr/bin/env node
// Get Copilot CLI remaining quota by driving interactive mode via node-pty.
// Prints JSON: {"remaining_pct": 64.6, "raw": "Remaining reqs.: 64.6%"}
// Exit code 0 on success, 1 on failure.

const pty = require('C:/Users/Administrator/AppData/Roaming/npm/node_modules/node-pty');

const TIMEOUT_MS = 20000;
const COPILOT_PATH = 'C:/Users/Administrator/AppData/Local/Microsoft/WinGet/Links/copilot.exe';

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
  // Once we see "Remaining reqs." in the status bar, we're done
  const m = buf.match(/Remaining reqs\.?: ?([\d.]+)%/);
  if (m && !done) {
    done = true;
    const clean = buf.replace(/\x1b\[[0-9;?]*[a-zA-Z]/g, '').replace(/\x1b\][^\x07]*\x07/g, '');
    const result = {
      ok: true,
      remaining_pct: parseFloat(m[1]),
      raw: m[0],
      ts: new Date().toISOString()
    };
    console.log(JSON.stringify(result));
    p.kill();
    process.exit(0);
  }
});

setTimeout(() => {
  if (!done) {
    console.log(JSON.stringify({ok: false, error: 'timeout', ts: new Date().toISOString()}));
    p.kill();
    process.exit(1);
  }
}, TIMEOUT_MS);
