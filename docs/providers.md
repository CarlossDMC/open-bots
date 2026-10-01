# Implementing a provider

A provider is an interchangeable adapter that runs inference for an agent. Runtime code only sees the `AgentProvider` trait and the `ProviderRegistry` (ADR 0003). This guide describes the standard shape every adapter follows. Read the provider checklist in [AGENTS.md](../AGENTS.md) first.

## The contract

`src-tauri/src/providers/mod.rs` defines `AgentProvider`:

| Method                        | Required                 | Purpose                                                                      |
| ----------------------------- | ------------------------ | ---------------------------------------------------------------------------- |
| `id`, `name`                  | Yes                      | Stable identifier stored on agents, and the display name.                    |
| `detect`                      | Yes                      | Installation and sign-in status. Report `unknown` when it cannot be proven.  |
| `capabilities`                | Yes                      | Only what the adapter actually implements and has verified.                  |
| `run_turn`                    | Yes                      | Runs one user turn and reports progress as `TurnEvent`s.                     |
| `list_models`                 | `model_selection`        | The model catalog shown in the model picker.                                 |
| `read_usage`                  | `usage_limits`           | Rate-limit windows shown in the usage panel. Read on demand only.            |
| `list_configured_mcp_servers` | `configured_mcp_servers` | Server names offered by the catalog import in Settings. Read on demand only. |

`run_turn` receives a `TurnRequest` (prompt, workspace, access, previous session, model, effort, runtime tools endpoint, and configured MCP servers) and sends events while it runs:

- `SessionStarted` as soon as the provider session id is known, so it is saved even if the turn fails later.
- `Message` for every complete agent message. Do not send partial text deltas; each one becomes a chat message.
- `ActionStarted` and `ActionCompleted` for tool calls, with a short summary such as the shell command.

It returns a `TurnOutcome` with the session id to resume next time, or an `AppError::Provider` with a user-readable reason. A cancelled turn returns `cancelled: true`, not an error.

### Runtime tools

An adapter that declares `runtime_tools` receives `TurnRequest.runtime_tools` with the URL of the local Open Bots MCP server and a per-turn bearer token. Register the server under the name in `RUNTIME_TOOLS_SERVER_NAME` (`open_bots`), so its tools appear as `mcp__open_bots__<tool>`. Pass the token only through the `RUNTIME_TOOLS_TOKEN_ENV` environment variable on `LineCommand.environment`, never as an argument. The token must reach the server as `Authorization: Bearer <token>`. Pre-approve the server's tools, because turns are non-interactive. Before you declare the capability, verify with a live turn that the CLI can call a tool; see `tests/live_runtime_tools.rs`, which is ignored by default because it makes real model calls.

### Configured MCP servers

An adapter that declares `configured_mcp_servers` implements `list_configured_mcp_servers`, which reads the names and health of the MCP servers in the provider's own configuration, and receives `TurnRequest.mcp_servers`: the servers the agent selected that are still in the global catalog. Read only names and statuses; never keep a server's command, URL, headers, or credentials. During a turn, load the provider's configuration next to the Open Bots server and pre-approve only the selected servers' tools; unselected servers must not be callable. Selected tools run without approval prompts, including writes to external services, so the UI says so next to every selection. The Claude Code adapter reads `claude mcp list`, uses `mcp__<server>__*` in `--allowed-tools`, and drops `--strict-mcp-config` only when servers are selected.

## CLI adapters

Most providers drive an official CLI. `src-tauri/src/providers/cli.rs` holds the shared scaffolding, so an adapter only writes what is specific to its CLI:

```rust
pub struct ExampleProvider {
    cli: CliProgram,
}

impl ExampleProvider {
    pub fn new(runner: Arc<dyn LineProcessRunner>) -> Self {
        Self { cli: CliProgram::new("example", "Example", runner) }
    }
}

#[async_trait]
impl AgentProvider for ExampleProvider {
    async fn detect(&self) -> AppResult<ProviderSummary> {
        self.cli
            .detect(self, SignInCheck {
                arguments: &["auth", "status"],
                confirmed: CliOutput::succeeded,
                login_hint: "example login",
            })
            .await
    }

    async fn run_turn(&self, request: TurnRequest, events: TurnEvents,
                      cancellation: CancellationSignal) -> AppResult<TurnOutcome> {
        self.cli
            .run_turn(turn_command(&request), request.session_id.clone(),
                      &mut ExampleStreamParser::default(), &events, cancellation)
            .await
    }
    // id, name, capabilities, and optional list_models / read_usage
}
```

What `CliProgram` provides:

- `detect` runs `<program> --version` and then the sign-in check. It produces the standard `available`, `not-installed`, and `unknown` summaries. Sign-in output is checked in memory and never logged.
- `run_turn` streams stdout lines through a `TurnStreamParser`, forwards events, and remembers the latest session id. It maps cancellation, `StreamEvent::Failed`, a missing program, and non-zero exits to the standard outcome and error messages.
- `run_quiet` runs a short command, with optional stdin, and collects its output. Use it for version, auth, catalog, and usage reads.
- `not_found` and `exit_error` give consistent error text, including a bounded stderr excerpt.

What the adapter writes:

- `turn_command`: the exact command line. Send the prompt through stdin, never as an argument. Map `WorkspaceAccess` to the CLI's own sandbox or tool restrictions, and pass model and effort as separate arguments.
- A `TurnStreamParser`: decode one output line into zero or more `StreamEvent`s. Ignore unknown and malformed lines so newer CLI versions do not break turns.
- Optional catalog and usage parsing. Tolerate missing fields. If nothing can be recognized, return an error rather than an empty success.

The adapter's module doc comment records the CLI version it was verified against, the exact flags, the event shapes, and any limitations. Examples are `codex.rs` and `claude.rs`.

## Registration

Register the adapter in `src-tauri/src/lib.rs` and export it from `providers/mod.rs`. The frontend needs no changes: provider lists, the model picker, and the usage panel all follow `detect` and `capabilities`.

## Tests

Adapter tests must not start real processes or make paid calls. `providers/testing.rs` provides a `ScriptedRunner` that replays recorded stdout lines and exits, one response per command, and records every command. Cover at least:

- exact command lines for new and resumed turns, each access level, and model and effort, with the prompt only on stdin;
- parser fixtures recorded from the real CLI, with account data removed, plus unknown and malformed lines;
- a full turn, a CLI-reported failure, a non-zero exit, cancellation, and a missing program;
- detection when not installed, signed in, and with unconfirmed sign-in;
- catalog and usage parsing, including unrecognized output.
