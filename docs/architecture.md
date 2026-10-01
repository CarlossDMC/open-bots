# Architecture

## Purpose

Open Bots is a local-first operating layer for persistent AI agents. The central boundary is that an agent is a durable domain entity while a provider is an interchangeable inference adapter.

## Layers

### Desktop UI

React renders local state, collects intent, and invokes narrow Tauri commands. It does not access SQLite, spawn processes, or contain agent runtime rules. A browser preview uses explicitly labeled demonstration data.

### Application

Application services coordinate repositories, policies, the event bus, and provider lookup. Tauri command handlers remain transport adapters around these services.

### Domain

The domain contains agents, tasks, events, approvals, artifacts, identity, permissions, and state transitions. It has no knowledge of desktop frameworks or infrastructure.

### Agent runtime

`ConversationService` runs one turn per user message. It records the message, moves the agent to `working`, and runs the provider turn in the background. It persists each reply and publishes `message.created`, `tool.*`, and `agent.started`/`completed`/`failed`/`cancelled` events. One turn runs per agent at a time, and Stop cancels it by killing the provider process. A provider session id is stored per agent and provider, so the next message resumes the same session. The first turn of a session carries a leading context block: a short runtime preamble (non-interactive turns, workspace, access level), then the agent's identity, instructions, and memories. Turns that were running when the application closed are marked failed on the next start.

`AgentRuntime` wakes agents without a user message. It subscribes to the event bus and turns events into durable wakes in `agent_wakes`:

- `routine.triggered` runs the routine's instructions.
- `task.assigned` wakes the assignee, unless the agent assigned the task to itself.
- `task.completed` and `task.failed` wake the agent that delegated the task.
- `agent.message` delivers a message from another agent.
- `approval.approved` and `approval.denied` wake the requester with the decision.

A busy agent's wakes queue and run one turn at a time once it is idle. They also survive restarts. Each wake records its notice as a `system` message in the conversation, so the user can see why the agent started. Every wake has a chain depth. User messages, routines, and approval decisions start at 0, and a wake caused by an agent's turn gets that turn's depth plus one. A wake that reaches `runtime.maxChainTurns` (setting, default 5) is skipped with a notice and an `agent.wake_skipped` event, so a loop between agents ends with the user. An agent that ends a turn with a pending approval request moves to `waiting` until the user decides. The runtime sleeps on the bus and never calls a model while waiting.

### Provider adapters

`AgentProvider` describes detection, capabilities, and `run_turn`, which streams provider-neutral `TurnEvent`s and honours a cancellation signal. `ProviderRegistry` locates adapters without provider conditionals in runtime code.

Two optional operations sit behind capabilities. A provider with `model_selection` lists its catalog through `list_models`, and `run_turn` receives the agent's model and reasoning effort. A provider with `usage_limits` reports account usage per rate-limit window through `read_usage`. `ProviderService` checks the capability before calling either one. Each agent stores a validated `ModelSelection`, where none means the provider default. `AgentService::update_model` changes it outside a running turn and publishes `agent.updated`. The new model applies from the next turn, and the provider session is kept.

- `MockProvider` is a deterministic development adapter that performs no model inference.
- `CodexProvider` drives the official Codex CLI through `codex exec --json` for new sessions and `codex exec resume --json` for follow-ups. The prompt goes through stdin. The model goes through `-m`, and the reasoning effort through the `model_reasoning_effort` config key. Models and usage limits are read from a short-lived `codex app-server` (`model/list`, `account/rateLimits/read`); see ADR 0006. The sandbox is `read-only` unless the agent has unattended shell and workspace-only filesystem access, in which case it is `workspace-write`. Detection runs `codex --version` and `codex login status`. Their output and Codex credentials are never read into application storage or logs. The adapter maps the documented JSONL events and ignores unknown ones. The verified CLI version and flags are recorded in `src-tauri/src/providers/codex.rs`.
- Providers with the `runtime_tools` capability connect to the Open Bots MCP server during a turn; see "Runtime tools" below and ADR 0008.
- `ClaudeProvider` drives the official Claude Code CLI through `claude -p --output-format stream-json --verbose`, adding `--resume <id>` for follow-ups. The prompt goes through stdin, the model through `--model`, and the reasoning effort through `--effort`. Models are a static list of CLI aliases. Usage limits come from the built-in `/usage` command, which the CLI answers locally without a model call; see ADR 0007. Tools are restricted with `--tools` and pre-approved with `--allowed-tools` under `--permission-mode dontAsk --permission-prompts none`, so a turn never waits for a prompt: read-only agents get `Read,Glob,Grep`, and workspace-write agents also get `Edit,Write,Bash`. Claude Code's Bash tool is not sandboxed by the operating system. Detection runs `claude --version` and `claude auth status --json` and reads only `loggedIn`. The verified CLI version and flags are recorded in `src-tauri/src/providers/claude.rs`.

CLI adapters share the scaffolding in `src-tauri/src/providers/cli.rs` (`CliProgram`, `TurnStreamParser`) for detection, the turn loop, cancellation, and error mapping, and supply only their command lines and output parsers; see [providers.md](providers.md).

Provider CLIs run through `LineProcessRunner` in process infrastructure, which streams stdout lines, keeps a bounded stderr tail for errors, and kills the process on cancellation. Stdio JSON-RPC servers run through `JsonRpcProcessClient`. It keeps stdin open, sends each request and waits for the response, skips notifications, enforces a timeout, and stops the server afterwards.

The desktop UI reads usage when the app starts, when the usage panel opens, a short debounce after each turn ends (`agent.completed`, `agent.failed`, `agent.cancelled`), and on manual refresh. It never polls on a timer.

Provider authentication should remain owned by official provider software whenever possible. Detection must report unknown state when authentication cannot be verified reliably.

### Events

`EventBus` is an in-process broadcast channel for structured domain events. Important events are also stored through the event repository for activity history. The desktop shell forwards every bus event to the frontend on the `runtime-event` channel so the activity timeline and approvals update live. Wakes that must survive a restart are stored by the agent runtime in `agent_wakes`. Other events are not replayed.

### Tools and approvals

Tools declare an identifier, description, input contract, required permission, action id, and structured result. `ApprovalPolicy` maps action context to `ALLOW`, `ASK`, or `DENY`. `ApprovalRequest` owns its transitions: a pending request resolves once, to approved or denied. `ApprovalService` persists requests and decisions and publishes `approval.requested`, `approval.approved`, and `approval.denied`.

### Runtime tools

The desktop process serves an MCP Streamable HTTP endpoint on `127.0.0.1` (ADR 0008). For each turn of a provider with `runtime_tools`, `ConversationService` issues a bearer token that maps to the agent and its chain depth and is revoked when the turn ends. The token reaches the CLI only through the `OPEN_BOTS_MCP_TOKEN` environment variable. `ToolService` evaluates each tool's action id with the approval policy, then runs it for the calling agent:

| Tool                                      | Effect                                                                                                                           |
| ----------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------- |
| `task_create`, `task_update`, `task_list` | Create and delegate tasks, change status and result, and list the agent's tasks. Only the assignee or creator can change a task. |
| `agent_list`, `agent_message`             | See the team and queue a message for another agent.                                                                              |
| `approval_request`                        | Record an approval request. The agent ends its turn and is woken with the decision.                                              |
| `memory_save`                             | Save a memory marked as learned by the agent.                                                                                    |

The policy allows these runtime actions. Unknown actions are `ASK`, and a tool that needs approval is reported as unsupported rather than run. The CLIs' own tools, such as file edits and shell, stay under each adapter's sandbox and permission flags. They are not routed through Open Bots approvals.

### Persistence

SQLite stores application state. SQL is restricted to repository implementations and migrations. Migrations are ordered scripts applied once each based on SQLite `user_version`. Artifacts will store metadata in SQLite and content as local files; only the domain boundary exists today. Agent memories are short local notes in `agent_memories`, at most 200 per agent, marked as written by the user or learned by the agent. They are sent with the agent context on the first turn of each provider session, so a memory saved mid-session reaches the provider in its next session.

### Routines

A routine is recurring work for one agent: a name, instructions, and an interval (5 minutes to 7 days) or a daily local time. Each agent can have up to 50. `Routine` owns the scheduling rules. Runs missed while the application was closed collapse into a single run on the next start, and re-enabling a paused routine schedules it from that moment. `run_routine_scheduler` sleeps until the earliest due routine, capped at five minutes so clock changes and system sleep are noticed, and wakes early when routines change. It reads local state only and never calls a model. A due routine publishes `routine.triggered`, and the agent runtime runs its instructions as a turn.

### Notifications

The frontend turns `approval.requested` and `routine.triggered` runtime events into desktop notifications through the official Tauri notification plugin, only while the window is in the background. The preference is stored per device in browser storage.

### System infrastructure

Process, output streaming, cancellation, Git, PTY, and scheduler capabilities live behind interfaces. Implementations must publish completion events instead of forcing an agent to poll while a long-running job executes.

### Application updates

The Tauri updater plugin checks a signed release manifest on GitHub and installs updates after user confirmation. It is distribution infrastructure and does not interact with agents, providers, or the runtime. See [ADR 0004](adr/0004-signed-self-updates.md).

## Dependency direction

```text
domain <- application <- commands
   ^           ^
   |           |
infrastructure adapters
```

Infrastructure implements domain/application contracts. Domain code never imports infrastructure details.

## Future distribution

Application services are designed as a boundary so the runtime may later move from the desktop process to a local daemon or remote worker. Local operation remains the default, and cloud identity must remain optional.

## Frontend design system

Visual values come from `src/styles/tokens.css` and are exposed as semantic Tailwind utilities. Theme state lives in `ThemeProvider` (`src/app/theme-provider.tsx`) with pure logic in `src/lib/theme.ts`. See [design-system.md](design-system.md) and [ADR 0005](adr/0005-design-tokens-and-motion.md).
