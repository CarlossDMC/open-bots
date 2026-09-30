# ADR 0001: Use Tauri 2 for the desktop application

- Status: Accepted
- Date: 2026-09-30

## Context

The application needs a cross-platform desktop shell with access to local processes, files, SQLite, and future PTY sessions while keeping a web-based UI development model.

## Decision

Use Tauri 2 with React, TypeScript, and Vite. Privileged operations live in Rust and are exposed through narrow commands and events.

## Consequences

The desktop bundle can remain smaller than a browser-bundled alternative, and Rust provides a suitable boundary for privileged operations. Contributors must install platform-specific WebView build prerequisites. UI code cannot assume browser-only deployment.
