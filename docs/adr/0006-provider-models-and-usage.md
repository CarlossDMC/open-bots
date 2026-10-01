# ADR 0006: Read provider models and usage limits through official channels

- Status: Accepted
- Date: 2026-10-01

## Context

People need to choose the model each agent runs with and to see how much of their provider allowance is left. The Codex CLI's `exec --json` stream reports neither the model catalog nor rate-limit windows. The official `codex app-server` stdio JSON-RPC protocol reports both through `model/list` and `account/rateLimits/read`. The CLI marks it as experimental. It was verified against `codex-cli 0.159.2` with `codex app-server generate-json-schema` and a live read. The server exits on end of input before answering, so a client must keep stdin open until the responses arrive.

## Decision

- `model_selection` and `usage_limits` are optional provider capabilities. Runtime code checks for them instead of branching on provider identifiers.
- An agent stores an optional model and reasoning effort, where none means the provider default. The domain validates both identifiers to a catalog-style character set before they reach a command line.
- Turns keep using `codex exec`. The model goes through `-m`, and the reasoning effort through the documented `model_reasoning_effort` config key.
- Catalog and usage reads start a short-lived `codex app-server` through a JSON-RPC process client in infrastructure. The client sends `initialize`, the `initialized` notification, and one request, then stops the server. Reads time out instead of hanging.
- Usage is read on demand and after each turn ends, never on a timer, so no provider process runs while agents are idle.
- Open Bots never reads Codex credentials or session files. Responses are not logged because they include account identifiers.

## Consequences

Protocol changes in a future Codex release can break these reads. Parsing tolerates missing fields, and failures show as an error instead of misleading numbers. The adapter's doc comment records the verified version. Only providers that implement and verify these capabilities show model pickers or usage. Other providers state that the feature is unsupported.
