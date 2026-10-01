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
  /** `null` keeps the provider's default model. */
  model?: string | null;
  reasoningEffort?: string | null;
  /** MCP servers from the provider's own configuration the agent may use, by name. */
  mcpServers?: string[];
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
  model?: string | null;
  reasoningEffort?: string | null;
}

export const taskStatuses = [
  "pending",
  "queued",
  "running",
  "waiting",
  "blocked",
  "completed",
  "failed",
  "cancelled"
] as const;
export type TaskStatus = (typeof taskStatuses)[number];

export interface AgentTask {
  id: string;
  title: string;
  description: string;
  status: TaskStatus;
  assignedAgentId?: string | null;
  /** The agent that created the task; absent when the user created it. */
  createdByAgentId?: string | null;
  parentTaskId?: string | null;
  workspaceId: string;
  /** What the assignee reported when the task finished. */
  result?: string | null;
  createdAt: string;
  updatedAt: string;
  startedAt?: string | null;
  completedAt?: string | null;
}

export interface NewTaskInput {
  title: string;
  description: string;
  assignedAgentId?: string;
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
  /** "agent" when the agent saved it through memory_save. */
  source: "user" | "agent";
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

/** Capabilities that unlock model and usage features; mirrors `ProviderCapability`. */
export const providerCapabilities = {
  modelSelection: "model_selection",
  usageLimits: "usage_limits",
  configuredMcpServers: "configured_mcp_servers"
} as const;

/** An MCP server the user made available to the agents of one provider. */
export interface McpCatalogEntry {
  providerId: string;
  name: string;
  addedAt: string;
}

export type McpServerStatus =
  "connected" | "needs-authentication" | "failed" | "pending-approval" | "unknown";

/** A server found in a provider's own configuration; only its name and health are read. */
export interface ConfiguredMcpServer {
  name: string;
  status: McpServerStatus;
}

export interface ProviderModel {
  id: string;
  displayName: string;
  description: string;
  isDefault: boolean;
  reasoningEfforts: string[];
  defaultReasoningEffort?: string | null;
}

export interface UsageWindow {
  durationMinutes?: number | null;
  /** Narrower allowance such as one model family; absent when all usage counts. */
  scope?: string | null;
  usedPercent: number;
  resetsAt?: string | null;
}

export interface ProviderUsage {
  providerId: string;
  plan?: string | null;
  windows: UsageWindow[];
  limitReached: boolean;
  checkedAt: string;
}

/** Usage for one provider, or the reason it could not be read. */
export interface ProviderUsageReport {
  providerId: string;
  providerName: string;
  usage?: ProviderUsage | null;
  error?: string | null;
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
  /** The peer agent that sent an incoming message. */
  sourceAgentId?: string | null;
  role: MessageRole;
  content: string;
  createdAt: string;
}

export interface RuntimeSettings {
  /** Turns that may chain from one user message or routine before the user steps in. */
  maxChainTurns: number;
}

/** Who wrote a group message or created a group. */
export type GroupAuthor =
  { kind: "user" } | { kind: "agent"; agentId: string } | { kind: "system" };

export interface RoundSpeaker {
  agentId: string;
  chainDepth: number;
}

/** Members still to answer, in speaking order; empty when the group is idle. */
export interface GroupRound {
  queue: RoundSpeaker[];
  activeWakeId?: string | null;
}

/** A conversation between the user and several agents about one topic. */
export interface Group {
  id: string;
  name: string;
  topic: string;
  /** Members in speaking order. */
  memberIds: string[];
  createdBy: GroupAuthor;
  round: GroupRound;
  createdAt: string;
  updatedAt: string;
}

export interface NewGroupInput {
  name: string;
  topic: string;
  memberIds: string[];
}

export interface GroupMessage {
  id: string;
  groupId: string;
  author: GroupAuthor;
  content: string;
  createdAt: string;
}
