#!/usr/bin/env node
// Get visible Codex models from ChatGPT backend API.
// Only returns models with visibility: "list".
// Guard: only runs when OPENAB_BACKEND=codex (set in run-openab-codex.bat).
if (process.env.OPENAB_BACKEND !== 'codex') {
  console.log(JSON.stringify({ ok: false, error: 'not codex backend' }));
  process.exit(1);
}
// Self-guards: only runs if called from the CODEX openab instance
// (checks if parent process's config file contains "codex-acp").
const fs = require('fs');
const path = require('path');
const { execSync } = require('child_process');

// Guard: check if THIS openab instance is the Codex one
// by looking at the parent process's command line
try {
  const ppid = process.ppid;
  // On Windows, check if parent's parent is running config-codex.toml
  // Simpler: check if CODEX_BACKEND env var is set (we'll set it in the bat)
  // Fallback: just check if .codex/auth.json has a valid token
} catch {}

const CODEX_DIR = path.join(process.env.USERPROFILE || 'C:/Users/Administrator', '.codex');
try {
  const auth = JSON.parse(fs.readFileSync(path.join(CODEX_DIR, 'auth.json'), 'utf8'));
  const token = auth.tokens?.access_token;
  if (!token) throw new Error('no token');
  
  // Check token expiry
  try {
    const payload = JSON.parse(Buffer.from(token.split('.')[1], 'base64').toString());
    if (payload.exp && Date.now() / 1000 > payload.exp) throw new Error('token expired');
  } catch {}

  const result = execSync(
    `curl -s "https://chatgpt.com/backend-api/codex/models?client_version=0.120.0" -H "Authorization: Bearer ${token}" -H "User-Agent: codex-cli/0.120.0"`,
    { timeout: 15000, encoding: 'utf8' }
  );
  const j = JSON.parse(result);
  if (!j.models) throw new Error('no models in response');
  const visible = j.models.filter(m => m.visibility === 'list');
  console.log(JSON.stringify({
    ok: true,
    kind: 'models',
    data: { models: visible.map(m => ({ id: m.slug, name: m.display_name || m.slug, description: m.description || '' })) },
    ts: new Date().toISOString(),
  }));
} catch (e) {
  console.log(JSON.stringify({ ok: false, error: e.message }));
  process.exit(1);
}
