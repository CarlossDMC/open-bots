# Contributing to Open Bots

Open Bots is early-stage software. Prefer focused changes with a clear reason over broad speculative frameworks.

AI coding agents and automated contributors must follow the repository-wide instructions in [AGENTS.md](AGENTS.md). Its architecture, safety, testing, and completion rules are also useful as a review checklist for human contributors.

## Setup

Install Node.js 20.19+, Rust stable, and the Tauri 2 prerequisites for your platform. Then run:

```bash
npm install
npm run tauri dev
```

## Branch workflow

Create a short-lived branch from `master`, keep commits focused, and open a pull request. Explain the user-visible behavior, architectural impact, validation performed, and remaining limitations.

## Required quality gates

Run before requesting review:

```bash
npm run format:check
npm run lint
npm run typecheck
npm test
npm run build
cd src-tauri
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
cargo check --all-targets --all-features
```

Use `npm run format` and `cargo fmt` to apply standard formatting. Tests should cover meaningful behavior such as state transitions, policy decisions, repository behavior, and adapter contracts. Avoid tests that only restate static markup.

## Architecture boundaries

- Domain code must not depend on Tauri, SQLite, provider CLIs, or shell commands.
- Tauri commands translate transport input and delegate to application services.
- UI components do not access the database or operating-system processes directly.
- Provider-specific behavior stays inside a provider adapter.
- Shell execution stays behind process, tool, or Git infrastructure boundaries.
- Credentials, tokens, cookies, and sensitive command input must never be logged.
- Local functionality must not require a cloud account.

## Proposing a provider

Open an issue before implementing a substantial adapter. Include:

- the official CLI or documented API used;
- supported platforms and detection behavior;
- authentication owned by the provider;
- verified capabilities and unsupported features;
- session, streaming, and cancellation semantics;
- how secrets remain local and out of logs.

Do not use web scraping, extracted cookies, session hijacking, undocumented authentication, or billing circumvention.

## Code style

Use English for source code, naming, comments, documentation, UI copy, errors, logs, tests, configuration, and commit messages. Keep files cohesive, errors actionable, and abstractions no larger than the responsibility they protect.
