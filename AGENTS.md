# Contribution Manual for Coding Agents

This file defines the working agreement for AI coding agents and automated contributors operating in this repository. Human contributors should also read [CONTRIBUTING.md](CONTRIBUTING.md).

## Project intent

Open Bots is a local-first, provider-agnostic runtime for persistent AI agents. It provides durable identity, state, events, tools, approvals, artifacts, workspaces, and orchestration around existing AI providers.

The project does not create a new language model, replace provider products, or bypass provider authentication, billing, or security controls.

The primary architectural invariant is:

```text
Agent Runtime != Model Provider
```

An agent is a persistent domain entity. A provider is an interchangeable adapter used for inference.

## Language

All repository content must be written in English, including:

- source code and identifiers;
- file and directory names;
- comments, logs, and error messages;
- UI copy and accessibility labels;
- tests and test descriptions;
- documentation and ADRs;
- configuration examples and environment variable names;
- commit messages and pull request suggestions.

Conversation with a contributor may use another language, but repository content must remain in English.

## Working principles

- Keep the application local-first. Core local functionality must not require an account, cloud service, telemetry system, or remote database.
- Keep the runtime provider-agnostic. Provider-specific behavior belongs in provider adapters.
- Prefer small, explicit, testable abstractions over speculative frameworks.
- Preserve existing user changes and avoid unrelated rewrites.
- Clearly label incomplete behavior as mock, placeholder, unsupported, or not implemented.
- Do not create fake integrations or imply that a provider capability works when it has not been verified.
- Use official provider CLIs or documented APIs. Do not scrape provider web interfaces or extract cookies, browser sessions, or private credentials.
- Keep secrets local when possible and never write credentials, tokens, cookies, or session data to logs.
- Avoid adding dependencies when the standard library or an existing dependency is sufficient.

## Repository map

```text
src/
  app/                  React application shell and navigation
  components/ui/        Shared UI primitives
  features/             Feature-focused UI
  hooks/                Shared React hooks
  lib/                  Desktop API bridge and utilities
  types/                Frontend contracts

src-tauri/src/
  application/          Use cases and orchestration
  commands/             Thin Tauri transport handlers
  domain/               Entities, value objects, and domain rules
  infrastructure/       SQLite, process, Git, and OS adapters
  providers/            Provider contracts, adapters, and registry
  runtime/              Event-driven runtime boundaries
  tools/                Tool contracts and registry

docs/
  adr/                   Important architectural decisions
```

## Dependency direction

Maintain this direction:

```text
domain <- application <- commands
   ^           ^
   |           |
infrastructure adapters
```

The domain must not import or depend conceptually on:

- Tauri;
- SQLite or `rusqlite`;
- React;
- shell commands;
- Codex, Claude, Gemini, or another concrete provider;
- filesystem or process implementations.

Application services coordinate domain behavior and ports. Infrastructure implements persistence and operating-system concerns. Commands only translate transport input and delegate to application services.

## Provider contributions

Provider-specific code must remain inside a dedicated adapter and be registered through `ProviderRegistry`.

Before implementing a provider:

1. Identify the official CLI or documented API.
2. Verify all commands and flags using official documentation or the installed CLI help.
3. Document supported platforms and detection behavior.
4. Report authentication as unknown when it cannot be verified reliably.
5. Declare only capabilities that the adapter actually implements.
6. Define session, resume, streaming, and cancellation behavior explicitly.
7. Keep provider credentials out of application storage and logs whenever possible.
8. Add contract-focused tests without making real paid model calls in the default test suite.

Runtime code must not contain provider identifier branches such as `if provider == "codex"`. It must query the registry and provider capabilities instead.

## Domain and runtime changes

Domain rules belong in domain types and should be tested directly. Do not place state transitions or approval rules in React components, Tauri handlers, or SQL repositories.

The runtime is event-driven. Agents should suspend while idle and wake for structured events. Do not add polling loops that repeatedly call a model while waiting for external work.

Use structured events with stable names, such as:

```text
agent.created
task.assigned
agent.message
tool.completed
process.completed
approval.resolved
```

Events that matter to the activity timeline or recovery must be persisted through an event repository. The in-process event bus alone is not durable.

## Persistence changes

- Keep SQL inside `src-tauri/src/infrastructure/database/` and migration files.
- UI components and Tauri handlers must never execute SQL directly.
- Add a migration for schema changes; do not silently mutate the existing schema for released versions.
- Repository tests should use an in-memory or temporary SQLite database.
- Store artifact metadata in SQLite and artifact content as local files unless a later ADR changes this decision.
- Do not add a heavy ORM without an accepted architectural reason.

## Processes, tools, Git, and PTY

- Shell and process execution must go through process or tool infrastructure boundaries.
- Git commands must remain behind `GitService`.
- Interactive processes must remain behind PTY boundaries.
- Long-running processes must publish completion events instead of blocking an agent execution loop.
- Every tool must define an identifier, description, input contract, required permission, and structured result.
- Potentially destructive or external actions must be evaluated by the approval policy before execution.

## UI contributions

The desktop UI should feel compact, calm, and native to a developer tool.

Prefer:

- dark mode as the primary experience;
- compact spacing and high information density;
- subtle borders and restrained colors;
- clear hierarchy and keyboard access;
- small reusable components;
- explicit loading, empty, and error states;
- Lucide icons instead of custom emoji or decorative glyphs.

Avoid:

- generic SaaS dashboard layouts;
- unnecessary charts, gradients, shadows, or oversized cards;
- giant headings and excessive rounded corners;
- using identity color as runtime status color;
- business logic or database access inside components;
- visual claims that an unsupported runtime capability is active.

Keep `AgentAvatar` reusable. Identity color, avatar variant, and runtime status are separate concepts.

## Error handling and logging

- Do not ignore errors or replace actionable errors with generic success states.
- Avoid `panic!`, `unwrap`, and `expect` in runtime paths. A top-level fatal startup failure may use `expect` when recovery is impossible and the message is actionable.
- Use typed errors at domain, application, and infrastructure boundaries.
- Logs should help local debugging without exposing prompts or secrets unnecessarily.
- Never log provider credentials, authentication tokens, cookies, private environment values, or full secret-bearing command lines.
- Use structured fields and the `debug`, `info`, `warn`, and `error` levels appropriately.

## Testing expectations

Add tests for behavior with real logic, especially:

- domain validation and state transitions;
- approval policy decisions;
- event publication and persistence;
- provider registry and adapter contracts;
- repository round trips;
- tool permission evaluation;
- process lifecycle behavior;
- utilities with branching or transformation logic.

Do not add tests solely to inflate coverage. Pure visual markup does not need a test unless it contains meaningful interaction or accessibility behavior.

Tests must be deterministic and must not require paid providers, user credentials, network access, or cloud services by default.

## Required quality gates

Run all applicable gates before declaring a change complete.

Frontend:

```bash
npm run format:check
npm run lint
npm run typecheck
npm test
npm run build
```

Rust:

```bash
cd src-tauri
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
cargo check --all-targets --all-features
```

Dependency audits when dependencies or lockfiles change:

```bash
npm audit --audit-level=high
cd src-tauri && cargo audit
```

Tauri compilation requires the platform prerequisites documented in the README. If the host lacks them, use CI or a representative container and clearly report where validation ran. Never claim a gate passed when it was not executed.

## Change workflow

1. Read the relevant code, tests, architecture documentation, and ADRs before editing.
2. Inspect the working tree and preserve unrelated user changes.
3. Define the smallest coherent change that satisfies the request.
4. Add or update tests alongside behavior changes.
5. Keep formatting and generated files consistent.
6. Run focused tests while iterating.
7. Run the full applicable quality gates before completion.
8. Review the final diff for coupling, duplication, leaked secrets, and accidental scope expansion.
9. Update README, architecture documentation, or an ADR only when behavior or an important decision changed.
10. Report functional behavior, mocked behavior, tests executed, and remaining limitations accurately.

## Architecture review checklist

Before completing a substantial change, verify:

- no provider-specific assumptions leaked into runtime code;
- no SQL exists outside persistence infrastructure;
- no shell or Git commands are scattered through domain or UI code;
- no domain logic moved into Tauri handlers or React components;
- no mandatory cloud dependency was introduced;
- no secret-bearing data is logged;
- no duplicate provider, tool, or event abstractions were added;
- no file or object accumulated unrelated responsibilities;
- no speculative abstraction was introduced without a current use;
- mock and unsupported behavior is labeled honestly;
- documentation matches the implemented behavior.

## Commits and pull requests

Use focused English commit messages. Conventional Commit style is preferred:

```text
feat: add persisted task assignment events
fix: reject invalid agent status transitions
docs: clarify provider adapter requirements
test: cover SQLite agent repository round trips
```

A pull request description should include:

- the problem and intended outcome;
- the implemented approach;
- important architectural tradeoffs;
- commands and tests executed;
- screenshots for meaningful UI changes;
- what remains mocked, unsupported, or deferred;
- migration or compatibility notes when relevant.

## Definition of done

A contribution is complete only when:

- the requested behavior is implemented without unrelated scope expansion;
- architectural boundaries remain intact;
- incomplete behavior is labeled accurately;
- relevant tests exist and pass;
- formatting, linting, type checking, Rust checks, and builds pass where applicable;
- documentation reflects user-visible or architectural changes;
- the final report distinguishes executed validation from unexecuted validation.
