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

`EventBus` is an in-process broadcast channel for structured domain events. Important events are also stored through the event repository for activity history. Durable wake-up delivery and replay are future work.

### Tools and approvals

Tools declare an identifier, description, input contract, required permission, and structured result. `ApprovalPolicy` maps action context to `ALLOW`, `ASK`, or `DENY`. Tool execution is not yet connected to an agent loop.

### Persistence

SQLite stores application state. SQL is restricted to repository implementations and migrations. Artifacts will store metadata in SQLite and content as local files; only the domain boundary exists today.

### System infrastructure

Process, output streaming, cancellation, Git, PTY, and scheduler capabilities live behind interfaces. Implementations must publish completion events instead of forcing an agent to poll while a long-running job executes.

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
