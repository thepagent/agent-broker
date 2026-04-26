#!/usr/bin/env node
/**
 * browser-bridge — HTTP bridge for agent-browser CLI.
 * Agents in containers call this via host.docker.internal:3002/message
 * using the same JSON-RPC pattern as flights-mcp.
 */
const http = require("http");
const { execFile } = require("child_process");
const { promisify } = require("util");
const exec = promisify(execFile);

const PORT = process.env.BROWSER_BRIDGE_PORT || 3002;
const AB = process.env.AGENT_BROWSER_PATH || "/opt/homebrew/bin/agent-browser";
const TIMEOUT = 60_000;

async function runAB(args) {
  const { stdout } = await exec(AB, args, { timeout: TIMEOUT, maxBuffer: 10 * 1024 * 1024 });
  return stdout;
}

const HANDLERS = {
  async browse_url({ url }) {
    await runAB(["open", url]);
    await runAB(["wait", "--load", "networkidle"]);
    const snap = await runAB(["snapshot", "-i", "-c"]);
    return snap;
  },
  async browse_snapshot({ selector }) {
    const args = ["snapshot", "-i", "-c"];
    if (selector) args.push("-s", selector);
    return await runAB(args);
  },
  async browse_click({ ref }) {
    await runAB(["click", ref]);
    await runAB(["wait", "500"]);
    return await runAB(["snapshot", "-i", "-c"]);
  },
  async browse_type({ ref, text }) {
    await runAB(["fill", ref, text]);
    return `Filled ${ref} with text`;
  },
  async browse_screenshot({}) {
    const path = `/tmp/ab-screenshot-${Date.now()}.png`;
    await runAB(["screenshot", path]);
    return `Screenshot saved to ${path}`;
  },
  async browse_get_text({ ref }) {
    return await runAB(["get", "text", ref]);
  },
  async browse_scroll({ direction, pixels }) {
    await runAB(["scroll", direction || "down", String(pixels || 500)]);
    return await runAB(["snapshot", "-i", "-c"]);
  },
  async browse_close({}) {
    await runAB(["close"]);
    return "Browser closed";
  },
};

const server = http.createServer(async (req, res) => {
  if (req.method === "GET" && req.url === "/health") {
    res.writeHead(200);
    return res.end("ok");
  }
  if (req.method !== "POST" || req.url !== "/message") {
    res.writeHead(404);
    return res.end("not found");
  }
  let body = "";
  for await (const chunk of req) body += chunk;
  try {
    const rpc = JSON.parse(body);
    const { name, arguments: args } = rpc.params || {};
    const handler = HANDLERS[name];
    if (!handler) {
      res.writeHead(200, { "Content-Type": "application/json" });
      return res.end(JSON.stringify({ jsonrpc: "2.0", id: rpc.id, error: { code: -32601, message: `Unknown tool: ${name}` } }));
    }
    const result = await handler(args || {});
    res.writeHead(200, { "Content-Type": "application/json" });
    res.end(JSON.stringify({ jsonrpc: "2.0", id: rpc.id, result: { content: [{ type: "text", text: String(result) }] } }));
  } catch (e) {
    res.writeHead(200, { "Content-Type": "application/json" });
    res.end(JSON.stringify({ jsonrpc: "2.0", id: 1, error: { code: -32000, message: e.message } }));
  }
});

server.listen(PORT, "0.0.0.0", () => console.log(`browser-bridge listening on :${PORT}`));
