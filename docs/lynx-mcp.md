# Lynx MCP

`lynx-mcp` exposes traffic already captured by a running Lynx Proxy to local MCP clients. It does not send business requests or persist captured traffic.

## Run locally

Start Lynx, then configure the MCP client to launch:

```json
{
  "mcpServers": {
    "lynx": {
      "command": "/absolute/path/to/lynx-mcp",
      "args": ["--lynx-url", "http://127.0.0.1:7788"]
    }
  }
}
```

When Lynx login is enabled, provide `LYNX_USER` and `LYNX_PASS` through the MCP client's environment configuration. `LYNX_TOKEN` can be used when a valid token is already available.

The MCP process starts collecting after it connects. It keeps at most 2,000 completed exchanges, 256 MB in total, and 2 MB per request or response body by default. All data is discarded when the process exits.

Available tools:

- `get_capture_status`
- `list_captured_requests`
- `search_captured_requests`
- `get_captured_request`
- `summarize_captured_traffic`
- `compare_captured_requests`
- `export_captured_request`

Sensitive headers and common secret query parameters are redacted from tool results.

To return raw captured credentials, cookies, tokens, query parameters, and body fields from every tool, start the server with:

```bash
lynx-mcp --allow-sensitive-data
```

This is a single global switch. Tool calls do not need an additional parameter.
