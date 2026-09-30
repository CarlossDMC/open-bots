import type {
  ActivityEvent,
  Agent,
  AgentTask,
  ApprovalRequest,
  ProviderSummary
} from "@/types/domain";

const defaultPermissions = {
  filesystem: "workspace-only",
  shell: "allowed",
  git: "approval-required",
  network: "restricted",
  browser: "denied"
} as const;

export const demoAgents: Agent[] = [
  {
    id: "demo-atlas",
    name: "Atlas",
    role: "Backend Engineer",
    description: "Designs durable services and keeps runtime boundaries explicit.",
    providerId: "mock",
    identityColor: "indigo",
    avatarVariant: "orbital",
    workspace: "~/projects/inventory",
    status: "working",
    instructions: "Prefer small, testable changes and document architectural decisions.",
    permissions: defaultPermissions,
    currentTask: "Implement inventory API",
    createdAt: "2026-01-10T10:00:00.000Z",
    updatedAt: "2026-01-10T10:54:00.000Z"
  },
  {
    id: "demo-nova",
    name: "Nova",
    role: "Frontend Engineer",
    description: "Builds focused interfaces with strong information hierarchy.",
    providerId: "mock",
    identityColor: "cyan",
    avatarVariant: "signal",
    workspace: "~/projects/inventory",
    status: "waiting",
    instructions: "Keep interactions keyboard-friendly and accessible.",
    permissions: defaultPermissions,
    currentTask: "Waiting for API contract",
    createdAt: "2026-01-10T10:02:00.000Z",
    updatedAt: "2026-01-10T10:53:00.000Z"
  },
  {
    id: "demo-orbit",
    name: "Orbit",
    role: "Research Agent",
    description: "Turns open questions into concise, sourced findings.",
    providerId: "mock",
    identityColor: "emerald",
    avatarVariant: "halo",
    workspace: "~/projects/inventory/research",
    status: "idle",
    instructions: "Separate verified facts from assumptions.",
    permissions: { ...defaultPermissions, shell: "denied" },
    createdAt: "2026-01-10T10:04:00.000Z",
    updatedAt: "2026-01-10T10:04:00.000Z"
  }
];

export const demoTasks: AgentTask[] = [
  {
    id: "task-1",
    title: "Implement inventory API",
    description: "Create the initial inventory endpoints.",
    status: "running",
    assignedAgentId: "demo-atlas",
    workspaceId: "inventory",
    createdAt: "2026-01-10T10:41:00.000Z",
    startedAt: "2026-01-10T10:42:00.000Z"
  },
  {
    id: "task-2",
    title: "Build inventory table",
    description: "Create the compact inventory list view.",
    status: "waiting",
    assignedAgentId: "demo-nova",
    workspaceId: "inventory",
    createdAt: "2026-01-10T10:43:00.000Z"
  },
  {
    id: "task-3",
    title: "Review sync strategies",
    description: "Compare local-first synchronization approaches.",
    status: "completed",
    assignedAgentId: "demo-orbit",
    workspaceId: "inventory",
    createdAt: "2026-01-10T09:30:00.000Z",
    completedAt: "2026-01-10T10:20:00.000Z"
  }
];

export const demoEvents: ActivityEvent[] = [
  {
    id: "event-1",
    type: "agent.started",
    subject: "Atlas",
    detail: "Agent started",
    occurredAt: "2026-01-10T10:42:00.000Z"
  },
  {
    id: "event-2",
    type: "task.assigned",
    subject: "Atlas",
    detail: "Task assigned: Implement inventory API",
    occurredAt: "2026-01-10T10:43:00.000Z"
  },
  {
    id: "event-3",
    type: "tool.started",
    subject: "Atlas",
    detail: "Shell command started",
    occurredAt: "2026-01-10T10:47:00.000Z"
  },
  {
    id: "event-4",
    type: "process.completed",
    subject: "Atlas",
    detail: "Test process completed successfully",
    occurredAt: "2026-01-10T10:51:00.000Z"
  },
  {
    id: "event-5",
    type: "artifact.created",
    subject: "Atlas",
    detail: "Created api-contract.json",
    occurredAt: "2026-01-10T10:52:00.000Z"
  },
  {
    id: "event-6",
    type: "agent.message",
    subject: "Atlas",
    detail: "Sent API contract to Nova",
    occurredAt: "2026-01-10T10:53:00.000Z"
  },
  {
    id: "event-7",
    type: "agent.waiting",
    subject: "Nova",
    detail: "Waiting for approval",
    occurredAt: "2026-01-10T10:54:00.000Z"
  }
];

export const demoApprovals: ApprovalRequest[] = [
  {
    id: "approval-1",
    agentId: "demo-atlas",
    agentName: "Atlas",
    action: "git push origin feature/inventory",
    reason: "Push the completed inventory implementation for review.",
    status: "pending",
    createdAt: "2026-01-10T10:55:00.000Z"
  }
];

export const demoProviders: ProviderSummary[] = [
  {
    id: "mock",
    name: "Mock Provider",
    kind: "mock",
    status: "available",
    detail: "Deterministic development adapter; no model calls are made.",
    capabilities: ["sessions", "structured_output"]
  },
  {
    id: "codex-cli",
    name: "Codex CLI",
    kind: "cli",
    status: "unknown",
    detail: "Adapter not implemented. Detection is intentionally unavailable.",
    capabilities: []
  },
  {
    id: "claude-code",
    name: "Claude Code",
    kind: "cli",
    status: "unknown",
    detail: "Adapter not implemented. Detection is intentionally unavailable.",
    capabilities: []
  }
];
