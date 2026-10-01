export const agentStatuses = [
  "idle",
  "working",
  "waiting",
  "paused",
  "failed",
  "completed"
] as const;
export type AgentStatus = (typeof agentStatuses)[number];

export const agentColors = ["indigo", "cyan", "emerald", "amber", "rose", "violet"] as const;
export type AgentColor = (typeof agentColors)[number];

export interface Agent {
  id: string;
  name: string;
  role: string;
  description: string;
  providerId: string;
  identityColor: AgentColor;
  avatarVariant: string;
  workspace: string;
  status: AgentStatus;
  instructions: string;
  permissions: AgentPermissions;
  currentTask?: string;
  createdAt: string;
  updatedAt: string;
}

export interface AgentPermissions {
  filesystem: "workspace-only" | "denied";
  shell: "allowed" | "approval-required" | "denied";
  git: "allowed" | "approval-required" | "denied";
  network: "allowed" | "restricted" | "denied";
  browser: "allowed" | "approval-required" | "denied";
}

export interface NewAgentInput {
  name: string;
  role: string;
  description: string;
  providerId: string;
  identityColor: AgentColor;
  workspace: string;
  instructions: string;
}

export type TaskStatus =
  "pending" | "queued" | "running" | "waiting" | "blocked" | "completed" | "failed" | "cancelled";

export interface AgentTask {
  id: string;
  title: string;
  description: string;
  status: TaskStatus;
  assignedAgentId?: string;
  parentTaskId?: string;
  workspaceId: string;
  createdAt: string;
  startedAt?: string;
  completedAt?: string;
}

export interface ActivityEvent {
  id: string;
  type: string;
  subject: string;
  detail: string;
  occurredAt: string;
}

/** A structured runtime event as persisted and published by the desktop runtime. */
export interface RuntimeEvent {
  id: string;
  eventType: string;
  aggregateId?: string | null;
  payload: Record<string, unknown>;
  occurredAt: string;
}

export type ApprovalStatus = "pending" | "approved" | "denied";
export type ApprovalDecision = Exclude<ApprovalStatus, "pending">;

export interface ApprovalRequest {
  id: string;
  agentId: string;
  action: string;
  reason: string;
  status: ApprovalStatus;
  createdAt: string;
  resolvedAt?: string | null;
}

export interface AgentMemory {
  id: string;
  agentId: string;
  content: string;
  createdAt: string;
}

export interface ProviderSummary {
  id: string;
  name: string;
  kind: "mock" | "cli" | "api";
  status: "available" | "not-installed" | "unknown";
  detail: string;
  capabilities: string[];
}

export type RoutineSchedule =
  { kind: "interval"; minutes: number } | { kind: "daily"; hour: number; minute: number };

export interface Routine {
  id: string;
  agentId: string;
  name: string;
  instructions: string;
  schedule: RoutineSchedule;
  enabled: boolean;
  nextRunAt: string;
  lastRunAt?: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface NewRoutineInput {
  agentId: string;
  name: string;
  instructions: string;
  schedule: RoutineSchedule;
}

export type MessageRole = "user" | "agent" | "system";

export interface ConversationMessage {
  id: string;
  agentId: string;
  role: MessageRole;
  content: string;
  createdAt: string;
}
