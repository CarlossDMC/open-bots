# ADR 0008: Offer runtime tools to providers through a local MCP server

- Status: Accepted
- Date: 2026-10-01

## Context

Agents need to act on Open Bots during a turn: create and delegate tasks, update task status, see the team, and save memories. Provider CLIs already run their own tool loop, so the runtime cannot add tools by intercepting model output. Both supported CLIs connect to Model Context Protocol servers through documented configuration:

- Claude Code 2.1.286: `--mcp-config <json>` with an HTTP server entry, `${VAR}` expansion in headers, and `--strict-mcp-config`. Tools are named `mcp__<server>__<tool>`, and `--allowed-tools mcp__<server>` pre-approves a server's tools.
- Codex 0.159.2: `-c mcp_servers.<name>.url`, `bearer_token_env_var`, and `default_tools_approval_mode` overrides on `exec` and `exec resume`. `exec` has no approval prompt, so without `approve` its MCP calls are rejected.

Both were verified with a live turn in which the CLI called `memory_save` on this server (`tests/live_runtime_tools.rs`, ignored by default because it makes real model calls). Codex exposes MCP tools inside its `exec` code environment and may leave them out of the tool description, so the agent context names the tools in full (`mcp__open_bots__<tool>`).

## Decision

- The desktop process hosts one MCP Streamable HTTP endpoint at `http://127.0.0.1:<random port>/mcp`. It accepts POST only and answers with JSON, with no SSE stream. It implements `initialize`, `ping`, `tools/list`, and `tools/call`.
- The endpoint is written directly on `hyper`, which Tauri already depends on, instead of adding an MCP SDK and web framework for five methods.
- Each provider turn gets a random bearer token. The token maps to the calling agent and its chain depth, and it is revoked when the turn ends. The token reaches the provider process only through the `OPEN_BOTS_MCP_TOKEN` environment variable. It never appears in arguments, logs, or `Debug` output.
- The server binds to loopback only. It rejects requests whose `Origin` header is not a loopback address, and bodies over 1 MiB.
- Tools implement the `Tool` contract, which covers id, description, input schema, required permission, and action id. `ToolService` evaluates the action id with the approval policy before executing. Runtime coordination actions are allowed, and unknown actions still need approval.
- Providers opt in with the `runtime_tools` capability. The runtime never branches on provider identifiers.

## Consequences

Agents can coordinate without a cloud service, and adding a provider that speaks MCP needs only adapter flags. The server lives only while the desktop app runs, so tools are unavailable when it is closed, as are turns. A local process that guesses a live token could act for that agent during its turn. Tokens carry 244 random bits and expire with the turn, which keeps that risk small. Future CLI releases may change MCP flags or how tools are surfaced. The live tests are the check to rerun after upgrading a CLI.
