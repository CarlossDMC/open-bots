import type { ActivityEvent, Agent, RuntimeEvent } from "@/types/domain";

const fixedDetails: Record<string, string> = {
  "agent.created": "Agent created",
  "agent.started": "Started working",
  "agent.completed": "Finished the turn",
  "agent.cancelled": "Stopped",
  "agent.wake_skipped": "Not woken: chained turn limit reached",
  "agent.waiting": "Waiting for an approval",
  "memory.added": "Memory added",
  "memory.removed": "Memory removed",
  "mcp_catalog.updated": "MCP server catalog updated"
};

const routineDetails: Record<string, string> = {
  "routine.created": "Routine created",
  "routine.updated": "Routine updated",
  "routine.deleted": "Routine deleted",
  "routine.triggered": "Routine triggered"
};

const taskDetails: Record<string, string> = {
  "task.created": "Task created",
  "task.assigned": "Task assigned",
  "task.started": "Task started",
  "task.updated": "Task updated",
  "task.completed": "Task completed",
  "task.failed": "Task failed",
  "task.cancelled": "Task cancelled"
};

const approvalDetails: Record<string, string> = {
  "approval.requested": "Requested approval",
  "approval.approved": "Approval granted",
  "approval.denied": "Approval denied"
};

/** Turns a structured runtime event into a timeline row with a readable subject and detail. */
export function toActivityEvent(event: RuntimeEvent, agents: Agent[]): ActivityEvent {
  return {
    id: event.id,
    type: event.eventType,
    subject: describeSubject(event, agents),
    detail: describeDetail(event),
    occurredAt: event.occurredAt
  };
}

/** Prepends a live event, ignoring duplicates already loaded from persistence. */
export function mergeRuntimeEvent(
  events: RuntimeEvent[],
  event: RuntimeEvent,
  limit: number
): RuntimeEvent[] {
  if (events.some((existing) => existing.id === event.id)) return events;
  return [event, ...events].slice(0, limit);
}

function describeSubject(event: RuntimeEvent, agents: Agent[]): string {
  const agentId = stringField(event.payload, "agentId") ?? event.aggregateId ?? undefined;
  const agent = agentId ? agents.find((candidate) => candidate.id === agentId) : undefined;
  // Only agent events carry the agent's own name; routine events carry the routine name.
  const payloadName = event.eventType.startsWith("agent.")
    ? stringField(event.payload, "name")
    : undefined;
  return agent?.name ?? payloadName ?? "Runtime";
}

function describeDetail(event: RuntimeEvent): string {
  const fixed = fixedDetails[event.eventType];
  if (fixed) return fixed;
  const approval = approvalDetails[event.eventType];
  if (approval) {
    const action = stringField(event.payload, "action");
    return action ? `${approval}: ${action}` : approval;
  }
  if (event.eventType === "message.created") {
    const role = stringField(event.payload, "role");
    return role === "user" ? "Received a message" : role === "agent" ? "Replied" : "Runtime notice";
  }
  if (event.eventType === "agent.message") {
    const from = stringField(event.payload, "fromName");
    return from ? `Message from ${from}` : "Message from another agent";
  }
  if (event.eventType === "agent.updated") {
    const servers = event.payload.mcpServers;
    if (Array.isArray(servers)) {
      return servers.length === 0
        ? "MCP servers cleared"
        : `MCP servers set to ${servers.filter((name) => typeof name === "string").join(", ")}`;
    }
    const model = stringField(event.payload, "model");
    if (!model) return "Model reset to provider default";
    const effort = stringField(event.payload, "reasoningEffort");
    return effort ? `Model set to ${model} (${effort})` : `Model set to ${model}`;
  }
  if (event.eventType === "agent.failed") {
    const detail = stringField(event.payload, "detail");
    return detail ? `Failed: ${detail}` : "Failed";
  }
  const task = taskDetails[event.eventType];
  if (task) {
    const title = stringField(event.payload, "title");
    return title ? `${task}: ${title}` : task;
  }
  const routine = routineDetails[event.eventType];
  if (routine) {
    const name = stringField(event.payload, "name");
    return name ? `${routine}: ${name}` : routine;
  }
  return stringField(event.payload, "detail") ?? event.eventType;
}

function stringField(payload: Record<string, unknown>, key: string): string | undefined {
  const value = payload[key];
  return typeof value === "string" && value.length > 0 ? value : undefined;
}
