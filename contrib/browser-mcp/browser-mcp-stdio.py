#!/usr/bin/env python3
"""Thin stdio MCP server that proxies browser tools to the browser-bridge on host."""
import json
import os
import sys
import urllib.request

BRIDGE_URL = os.environ.get("BROWSER_BRIDGE_URL", "http://127.0.0.1:3002/message")

TOOLS = [
    {
        "name": "browse_url",
        "description": "Open a URL in a real browser and return the accessibility snapshot (interactive elements with refs). Use this when web_fetch fails on JS-heavy pages.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "url": {"type": "string", "description": "URL to open"},
            },
            "required": ["url"],
        },
    },
    {
        "name": "browse_snapshot",
        "description": "Get the current page's accessibility snapshot (interactive elements with @refs). Optionally scope to a CSS selector.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "selector": {"type": "string", "description": "Optional CSS selector to scope the snapshot"},
            },
        },
    },
    {
        "name": "browse_click",
        "description": "Click an element by its @ref from the snapshot, then return updated snapshot.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "ref": {"type": "string", "description": "Element ref from snapshot, e.g. @e5"},
            },
            "required": ["ref"],
        },
    },
    {
        "name": "browse_type",
        "description": "Clear and fill a text field by its @ref.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "ref": {"type": "string", "description": "Element ref, e.g. @e3"},
                "text": {"type": "string", "description": "Text to type"},
            },
            "required": ["ref", "text"],
        },
    },
    {
        "name": "browse_screenshot",
        "description": "Take a screenshot of the current page.",
        "inputSchema": {"type": "object", "properties": {}},
    },
    {
        "name": "browse_get_text",
        "description": "Get the text content of an element by its @ref.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "ref": {"type": "string", "description": "Element ref, e.g. @e1"},
            },
            "required": ["ref"],
        },
    },
    {
        "name": "browse_scroll",
        "description": "Scroll the page in a direction, then return updated snapshot.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "direction": {"type": "string", "description": "up, down, left, or right", "default": "down"},
                "pixels": {"type": "integer", "description": "Pixels to scroll (default 500)", "default": 500},
            },
        },
    },
    {
        "name": "browse_close",
        "description": "Close the browser session.",
        "inputSchema": {"type": "object", "properties": {}},
    },
]

KNOWN_TOOLS = {t["name"] for t in TOOLS}


def send(id, result=None, error=None):
    msg = {"jsonrpc": "2.0", "id": id}
    if error:
        msg["error"] = error
    else:
        msg["result"] = result
    sys.stdout.write(json.dumps(msg) + "\n")
    sys.stdout.flush()


def proxy_to_bridge(tool_name, args):
    payload = json.dumps({
        "jsonrpc": "2.0", "id": 1,
        "method": "tools/call",
        "params": {"name": tool_name, "arguments": args},
    }).encode()
    req = urllib.request.Request(BRIDGE_URL, data=payload, headers={"Content-Type": "application/json"})
    with urllib.request.urlopen(req, timeout=90) as resp:
        return json.loads(resp.read())


def main():
    for line in sys.stdin:
        line = line.strip()
        if not line:
            continue
        try:
            msg = json.loads(line)
        except json.JSONDecodeError:
            continue

        method = msg.get("method", "")
        mid = msg.get("id")

        if method == "initialize":
            send(mid, {"protocolVersion": "2024-11-05", "capabilities": {"tools": {"listChanged": False}}, "serverInfo": {"name": "browser-mcp", "version": "1.0.0"}})
        elif method == "notifications/initialized":
            pass
        elif method == "tools/list":
            send(mid, {"tools": TOOLS})
        elif method == "tools/call":
            params = msg.get("params", {})
            name = params.get("name")
            args = params.get("arguments", {})
            if name not in KNOWN_TOOLS:
                send(mid, error={"code": -32601, "message": f"Unknown tool: {name}"})
                continue
            try:
                resp = proxy_to_bridge(name, args)
                if "error" in resp:
                    send(mid, {"content": [{"type": "text", "text": f"Bridge error: {resp['error'].get('message', 'unknown')}"}], "isError": True})
                else:
                    send(mid, resp.get("result", {"content": [{"type": "text", "text": "No result"}]}))
            except Exception as e:
                send(mid, {"content": [{"type": "text", "text": f"Browser bridge unreachable: {e}"}], "isError": True})
        elif mid is not None:
            send(mid, error={"code": -32601, "message": f"Unknown method: {method}"})


if __name__ == "__main__":
    main()
