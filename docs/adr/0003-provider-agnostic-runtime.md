# ADR 0003: Separate the agent runtime from model providers

- Status: Accepted
- Date: 2026-09-30

## Context

An agent has identity, state, permissions, tasks, memory, and a workspace that should survive changes in the model or vendor used for inference.

## Decision

The runtime depends on the `AgentProvider` contract and resolves implementations through `ProviderRegistry`. Provider-specific detection, capabilities, session semantics, request translation, and cancellation remain inside adapters.

## Consequences

Runtime code must not branch on provider identifiers. Capabilities are queried rather than assumed. Adapters may expose different feature sets without changing agent identity. Some provider features may remain unavailable until their adapter implements and verifies them.
