# ADR 0002: Keep core operation local-first

- Status: Accepted
- Date: 2026-09-30

## Context

Persistent agents need durable state and access to providers already installed and authenticated on the user's machine. A mandatory service would add operational cost, credential risk, and account friction.

## Decision

Core application state uses local SQLite and local workspaces. No login, cloud backend, remote database, or telemetry is required. A future account may enable optional sync, remote workers, backups, or teams.

## Consequences

The application works offline except when a selected provider or tool requires a network. Backup and multi-device synchronization are user responsibilities until optional cloud features exist. New features must preserve useful local operation without an account.
