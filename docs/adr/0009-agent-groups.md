# ADR 0009: Run group conversations as sequential rounds of durable wakes

- Status: Accepted
- Date: 2026-10-01

## Context

Users want to bring several agents together around one topic: a group that sits in the sidebar next to the agents' own conversations, where the user and the members talk. Agents should also be able to open such groups themselves. Until now, each agent had exactly one conversation and one provider session per provider, and agents could reach each other only through `agent_message` notices in their own conversations.

Two runtime facts constrain the design. An agent runs one turn at a time, because its workspace and provider CLI are shared. And every turn that an agent causes must count toward `runtime.maxChainTurns`, so that agents cannot loop without the user.

## Decision

- A group is a domain entity with a name, a topic, ordered members (1 to 12), and a round: the queue of members still to answer. Group messages live in their own table, attributed to the user, a member, or the runtime.
- A message queues the members who answer. A user message queues everyone, or only the members it mentions as `@Name`, and starts at chain depth 0. A member's reply is posted to the group as it streams. It wakes only the members it mentions, one chain level deeper.
- Members answer one at a time, in order, so each one sees the replies before it. Each answer is a durable `GroupTurn` wake in `agent_wakes` for the current speaker. A busy member therefore answers after its current turn, and a round survives restarts. The round records the id of the wake it queued. A wake that is no longer current, for example after Stop or a restart, is discarded.
- Each group has its own provider session per agent. Provider sessions are keyed by agent, provider, and conversation (`''` for the agent's direct conversation). A member's first turn in a group carries the usual leading context, plus the group's topic, members, and mention rules. Later turns send only the messages that member has not seen yet.
- Agents use the `group_create`, `group_list`, and `group_post` runtime tools (action ids `group.read` and `group.write`, allowed by the policy). An agent that creates a group always joins it. Groups an agent creates appear in the sidebar without approval.

## Consequences

Groups reuse the wake queue, the chain limit, and the event bus instead of adding a second scheduler. Sequential answers are slower than parallel ones, but they keep the one-turn-per-agent rule and give members a coherent discussion. A group round is moved on by turn-end events. If the event bus drops such an event under heavy lag, that round stalls until the user stops it or the application restarts. Resetting an agent's session clears its group sessions too. Editing a group's members or topic is not supported yet.
