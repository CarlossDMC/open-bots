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

The future runtime will wake from structured events, restore state, request a provider action, execute an approved tool, persist results, and either continue or suspend. The current code provides the event bus and boundaries only; the execution loop is not implemented.

### Provider adapters

`AgentProvider` describes detection, capabilities, session startup, requests, and cancellation. `ProviderRegistry` locates adapters without provider conditionals in runtime code. The only current implementation is `MockProvider`, a deterministic development adapter that performs no model inference.

Provider authentication should remain owned by official provider software whenever possible. Detection must report unknown state when authentication cannot be verified reliably.

### Events

`EventBus` is an in-process broadcast channel for structured domain events. Important events are also stored through the event repository for activity history. The desktop shell forwards every bus event to the frontend on the `runtime-event` channel so the activity timeline and approvals update live. Durable wake-up delivery and replay are future work.

### Tools and approvals

Tools declare an identifier, description, input contract, required permission, and structured result. `ApprovalPolicy` maps action context to `ALLOW`, `ASK`, or `DENY`. `ApprovalRequest` owns its transitions: a pending request resolves once, to approved or denied. `ApprovalService` persists requests and decisions and publishes `approval.requested`, `approval.approved`, and `approval.denied`. Tool execution is not yet connected to an agent loop, so nothing raises approval requests at runtime yet.

### Persistence

SQLite stores application state. SQL is restricted to repository implementations and migrations. Migrations are ordered scripts applied once each based on SQLite `user_version`. Artifacts will store metadata in SQLite and content as local files; only the domain boundary exists today. Agent memories are short local notes in `agent_memories`; they will be added to provider system instructions once sessions exist.

### Routines

A routine is recurring work for one agent: a name, instructions, and an interval (5 minutes to 7 days) or a daily local time. Each agent can have up to 50. `Routine` owns the scheduling rules. Runs missed while the application was closed collapse into a single run on the next start, and re-enabling a paused routine schedules it from that moment. `run_routine_scheduler` sleeps until the earliest due routine, capped at five minutes so clock changes and system sleep are noticed, and wakes early when routines change. It reads local state only and never calls a model. A due routine publishes `routine.triggered`; agents do not execute routine instructions until provider sessions exist.

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
