---
paths:
  - "src/mcp.rs"
  - "src/mcp/**"
  - "src/server/tools.rs"
  - "e2e/tests/api-mcp.spec.ts"
---

# MCP endpoint

`src/mcp.rs` (auth and dispatch) + `src/mcp/rpc.rs` (JSON-RPC responses, `RpcError`), `POST /mcp`:

- Streamable HTTP, stateless JSON-RPC 2.0 (`initialize`, `ping`,
  `tools/list`, `tools/call`; notifications get 202). Every registered
  `ApiAction` is a tool named after its id (`get_profile`), with its request
  schema as `inputSchema` and response schema as `outputSchema`.
- Only tokens issued for `TokenAudience::Mcp` ("Used by: AI assistants" on
  the Developer tab) and sign-in tokens from connected apps
  (`connected-apps.md`) work here, and they don't work on `/api/v1`.
  `tools/list` shows only the tools the token's permissions allow; without
  a token only public tools (`public_tool`), and other calls get 401 with
  `resource_metadata`.
- Tool problems are results with `isError: true` and the problem+json body
  as text (so the model can read `code` and `detail`); bad arguments and
  unknown tools are JSON-RPC `-32602` errors.
