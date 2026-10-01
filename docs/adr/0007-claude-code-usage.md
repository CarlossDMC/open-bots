# ADR 0007: Read Claude Code usage limits from the local `/usage` command

- Status: Accepted
- Date: 2026-10-01

## Context

Claude Code has no structured, on-demand usage command. Two channels were verified against `2.1.286 (Claude Code)`:

- During a turn, `stream-json` emits a `rate_limit_event` with structured utilization and reset times. It exists only while a turn runs, so the usage panel would stay empty until the first turn after every start.
- The built-in `/usage` command, sent on stdin to `claude -p`, is answered locally. It makes no model call, finishes in about two seconds, and reports the five-hour session window, the weekly window, and per-model weekly windows as human-readable text.

## Decision

- `ClaudeProvider::read_usage` sends `/usage` to `claude -p --safe-mode --output-format stream-json --verbose --no-session-persistence --tools ""`. `--safe-mode` keeps user hooks and plugins out of the read, and no session is saved.
- The adapter parses `Current session` as a five-hour window and `Current week (<scope>)` as a weekly window. `all models` has no scope, and any other scope, such as `Fable`, is reported through the new optional `UsageWindow.scope`.
- Reset times are printed in the system time zone without a year, so they are read as local time on the next matching date.
- Unrecognized lines are skipped. A report with no recognizable window is an error, not an empty success.
- The plan comes from `subscriptionType` in `claude auth status --json`. No other account field is read.
- Reads follow ADR 0006: on demand and after turns, never on a timer.

## Consequences

The text format is not a stable interface. A wording change in a future release turns usage into a visible error instead of wrong numbers, and the adapter documents the verified version. If Claude Code adds structured usage output, the adapter should switch to it.
