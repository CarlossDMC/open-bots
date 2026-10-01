import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  demoAgents,
  demoApprovals,
  demoEvents,
  demoProviders,
  demoTasks,
  demoUsageReports
} from "@/lib/demo-data";
import type { ResolvedTheme } from "@/lib/theme";
import type {
  Agent,
  AgentMemory,
  AgentTask,
  ConfiguredMcpServer,
  ConversationMessage,
  ApprovalDecision,
  Group,
  GroupMessage,
  ApprovalRequest,
  McpCatalogEntry,
  NewAgentInput,
  NewGroupInput,
  NewRoutineInput,
  NewTaskInput,
  ProviderModel,
  ProviderSummary,
  ProviderUsageReport,
  Routine,
  RuntimeEvent,
  RuntimeSettings,
  TaskStatus
} from "@/types/domain";

/** Mirrors `RUNTIME_EVENT_CHANNEL` in `src-tauri/src/lib.rs`. */
const runtimeEventChannel = "runtime-event";

/** Browser preview only: memories live in this tab and are lost on reload. */
const demoMemories = new Map<string, AgentMemory[]>();

export function isTauriRuntime(): boolean {
  return "__TAURI_INTERNALS__" in window;
}

export async function listAgents(): Promise<{ agents: Agent[]; isDemo: boolean }> {
  if (!isTauriRuntime()) return { agents: demoAgents, isDemo: true };
  return { agents: await invoke<Agent[]>("list_agents"), isDemo: false };
}

export async function createAgent(input: NewAgentInput, mcpServers: string[] = []): Promise<Agent> {
  if (!isTauriRuntime()) {
    const timestamp = new Date().toISOString();
    return {
      ...input,
      id: crypto.randomUUID(),
      avatarVariant: "orbital",
      status: "idle",
      mcpServers,
      permissions: {
        filesystem: "workspace-only",
        shell: "allowed",
        git: "approval-required",
        network: "allowed",
        browser: "denied"
      },
      createdAt: timestamp,
      updatedAt: timestamp
    };
  }
  return invoke<Agent>("create_agent", { input, mcpServers });
}

export async function listProviders(): Promise<ProviderSummary[]> {
  if (!isTauriRuntime()) return demoProviders;
  return invoke<ProviderSummary[]>("list_providers");
}

/** The browser preview has no provider catalog, so it lists no models. */
export async function listProviderModels(providerId: string): Promise<ProviderModel[]> {
  if (!isTauriRuntime()) return [];
  return invoke<ProviderModel[]>("list_provider_models", { providerId });
}

/** The browser preview reads no provider accounts; it shows labeled demonstration usage. */
export async function readProviderUsage(): Promise<ProviderUsageReport[]> {
  if (!isTauriRuntime()) return demoUsageReports();
  return invoke<ProviderUsageReport[]>("read_provider_usage");
}

/** `null` model restores the provider default. Applies from the agent's next turn. */
export async function updateAgentModel(
  agent: Agent,
  model: string | null,
  reasoningEffort: string | null
): Promise<Agent> {
  if (!isTauriRuntime()) {
    return { ...agent, model, reasoningEffort, updatedAt: new Date().toISOString() };
  }
  return invoke<Agent>("update_agent_model", { agentId: agent.id, model, reasoningEffort });
}

/** Replaces the agent's configured MCP servers. Applies from the agent's next turn. */
/** Deletes the agent with its conversation, memories, routines, and group memberships. */
export async function deleteAgent(agentId: string): Promise<void> {
  if (!isTauriRuntime()) throw new Error("Deleting agents requires the desktop runtime.");
  await invoke("delete_agent", { agentId });
}

/** Turns workspace writes and internet access on or off from the agent's next turn. */
export async function updateAgentAccess(
  agent: Agent,
  workspaceWrite: boolean,
  network: boolean
): Promise<Agent> {
  if (!isTauriRuntime()) {
    return {
      ...agent,
      permissions: {
        ...agent.permissions,
        filesystem: "workspace-only",
        shell: workspaceWrite ? "allowed" : "approval-required",
        network: network ? "allowed" : "restricted"
      },
      updatedAt: new Date().toISOString()
    };
  }
  return invoke<Agent>("update_agent_access", { agentId: agent.id, workspaceWrite, network });
}

export async function updateAgentMcpServers(agent: Agent, servers: string[]): Promise<Agent> {
  if (!isTauriRuntime()) {
    return { ...agent, mcpServers: servers, updatedAt: new Date().toISOString() };
  }
  return invoke<Agent>("update_agent_mcp_servers", { agentId: agent.id, servers });
}

/** The browser preview has no provider configuration, so the catalog starts empty. */
export async function listMcpCatalog(): Promise<McpCatalogEntry[]> {
  if (!isTauriRuntime()) return [];
  return invoke<McpCatalogEntry[]>("list_mcp_catalog");
}

/** Runs the provider's own server listing; nothing is saved until `saveMcpCatalog`. */
export async function discoverMcpServers(providerId: string): Promise<ConfiguredMcpServer[]> {
  if (!isTauriRuntime()) {
    throw new Error("Importing MCP servers needs the desktop app.");
  }
  return invoke<ConfiguredMcpServer[]>("discover_mcp_servers", { providerId });
}

/** Replaces one provider's catalog entries and returns the whole catalog. */
export async function saveMcpCatalog(
  providerId: string,
  servers: string[]
): Promise<McpCatalogEntry[]> {
  if (!isTauriRuntime()) {
    throw new Error("Saving MCP servers needs the desktop app.");
  }
  return invoke<McpCatalogEntry[]>("save_mcp_catalog", { providerId, servers });
}

export async function listEvents(limit: number): Promise<RuntimeEvent[]> {
  if (!isTauriRuntime()) return demoEvents;
  return invoke<RuntimeEvent[]>("list_events", { limit });
}

/** Subscribes to live runtime events. Resolves to an unsubscribe function. */
export async function onRuntimeEvent(handler: (event: RuntimeEvent) => void): Promise<() => void> {
  if (!isTauriRuntime()) return () => undefined;
  return listen<RuntimeEvent>(runtimeEventChannel, (message) => handler(message.payload));
}

export async function listApprovals(): Promise<ApprovalRequest[]> {
  if (!isTauriRuntime()) return demoApprovals;
  return invoke<ApprovalRequest[]>("list_approvals");
}

export async function resolveApproval(
  approval: ApprovalRequest,
  decision: ApprovalDecision
): Promise<ApprovalRequest> {
  if (!isTauriRuntime()) {
    return { ...approval, status: decision, resolvedAt: new Date().toISOString() };
  }
  return invoke<ApprovalRequest>("resolve_approval", { id: approval.id, decision });
}

export async function listMemories(agentId: string): Promise<AgentMemory[]> {
  if (!isTauriRuntime()) return demoMemories.get(agentId) ?? [];
  return invoke<AgentMemory[]>("list_memories", { agentId });
}

export async function addMemory(agentId: string, content: string): Promise<AgentMemory> {
  if (!isTauriRuntime()) {
    const memory: AgentMemory = {
      id: crypto.randomUUID(),
      agentId,
      content: content.trim(),
      source: "user",
      createdAt: new Date().toISOString()
    };
    demoMemories.set(agentId, [memory, ...(demoMemories.get(agentId) ?? [])]);
    return memory;
  }
  return invoke<AgentMemory>("add_memory", { agentId, content });
}

export async function removeMemory(memory: AgentMemory): Promise<void> {
  if (!isTauriRuntime()) {
    const remaining = (demoMemories.get(memory.agentId) ?? []).filter(
      (candidate) => candidate.id !== memory.id
    );
    demoMemories.set(memory.agentId, remaining);
    return;
  }
  await invoke("remove_memory", { id: memory.id });
}

/** Tasks are persisted by the desktop runtime; the browser preview shows sample tasks. */
export const tasksUnavailableMessage = "Managing tasks requires the desktop runtime.";

export async function listTasks(): Promise<AgentTask[]> {
  if (!isTauriRuntime()) return demoTasks;
  return invoke<AgentTask[]>("list_tasks");
}

export async function createTask(input: NewTaskInput): Promise<AgentTask> {
  if (!isTauriRuntime()) throw new Error(tasksUnavailableMessage);
  return invoke<AgentTask>("create_task", { input });
}

export async function updateTaskStatus(
  id: string,
  status: TaskStatus,
  result?: string
): Promise<AgentTask> {
  if (!isTauriRuntime()) throw new Error(tasksUnavailableMessage);
  return invoke<AgentTask>("update_task_status", { id, status, result: result ?? null });
}

/** Browser preview only: mirrors the runtime default. */
const demoRuntimeSettings: RuntimeSettings = { maxChainTurns: 5 };

export async function getRuntimeSettings(): Promise<RuntimeSettings> {
  if (!isTauriRuntime()) return demoRuntimeSettings;
  return invoke<RuntimeSettings>("get_runtime_settings");
}

export async function updateRuntimeSettings(input: RuntimeSettings): Promise<RuntimeSettings> {
  if (!isTauriRuntime()) throw new Error("Runtime settings require the desktop runtime.");
  return invoke<RuntimeSettings>("update_runtime_settings", { input });
}

/** Routines are scheduled by the desktop runtime; the browser preview cannot run them. */
export const routinesUnavailableMessage = "Routines require the desktop runtime.";

export async function listRoutines(agentId: string): Promise<Routine[]> {
  if (!isTauriRuntime()) return [];
  return invoke<Routine[]>("list_routines", { agentId });
}

export async function createRoutine(input: NewRoutineInput): Promise<Routine> {
  if (!isTauriRuntime()) throw new Error(routinesUnavailableMessage);
  return invoke<Routine>("create_routine", { input });
}

export async function setRoutineEnabled(id: string, enabled: boolean): Promise<Routine> {
  if (!isTauriRuntime()) throw new Error(routinesUnavailableMessage);
  return invoke<Routine>("set_routine_enabled", { id, enabled });
}

export async function deleteRoutine(id: string): Promise<void> {
  if (!isTauriRuntime()) throw new Error(routinesUnavailableMessage);
  await invoke("delete_routine", { id });
}

/** Provider turns run in the desktop runtime; the browser preview cannot start them. */
export const messagingUnavailableMessage = "Messaging requires the desktop runtime.";

export async function listMessages(agentId: string): Promise<ConversationMessage[]> {
  if (!isTauriRuntime()) return [];
  return invoke<ConversationMessage[]>("list_messages", { agentId });
}

/** Records the message and starts a turn. Replies arrive as runtime events. */
export async function sendMessage(agentId: string, content: string): Promise<ConversationMessage> {
  if (!isTauriRuntime()) throw new Error(messagingUnavailableMessage);
  return invoke<ConversationMessage>("send_message", { agentId, content });
}

export async function cancelTurn(agentId: string): Promise<void> {
  if (!isTauriRuntime()) throw new Error(messagingUnavailableMessage);
  await invoke("cancel_turn", { agentId });
}

/** Deletes every message in the agent's own conversation and starts its next turn fresh. */
export async function clearConversation(agentId: string): Promise<void> {
  if (!isTauriRuntime()) throw new Error(messagingUnavailableMessage);
  await invoke("clear_conversation", { agentId });
}

/** Starts a fresh provider session on the agent's next turn; the conversation stays visible. */
export async function resetAgentSession(agentId: string): Promise<void> {
  if (!isTauriRuntime()) throw new Error(messagingUnavailableMessage);
  await invoke("reset_agent_session", { agentId });
}

/** Groups run in the desktop runtime; the browser preview has none. */
export const groupsUnavailableMessage = "Groups require the desktop runtime.";

export async function listGroups(): Promise<Group[]> {
  if (!isTauriRuntime()) return [];
  return invoke<Group[]>("list_groups");
}

export async function createGroup(input: NewGroupInput): Promise<Group> {
  if (!isTauriRuntime()) throw new Error(groupsUnavailableMessage);
  return invoke<Group>("create_group", { input });
}

export async function deleteGroup(groupId: string): Promise<void> {
  if (!isTauriRuntime()) throw new Error(groupsUnavailableMessage);
  await invoke("delete_group", { groupId });
}

/** Deletes the group's messages and starts its members' sessions there over. */
export async function clearGroup(groupId: string): Promise<void> {
  if (!isTauriRuntime()) throw new Error(groupsUnavailableMessage);
  await invoke("clear_group", { groupId });
}

export async function listGroupMessages(groupId: string): Promise<GroupMessage[]> {
  if (!isTauriRuntime()) return [];
  return invoke<GroupMessage[]>("list_group_messages", { groupId });
}

/** Records the message and queues the members who answer. Replies arrive as runtime events. */
export async function sendGroupMessage(groupId: string, content: string): Promise<GroupMessage> {
  if (!isTauriRuntime()) throw new Error(groupsUnavailableMessage);
  return invoke<GroupMessage>("send_group_message", { groupId, content });
}

/** Ends the group's round: the member answering now is stopped and nobody else answers. */
export async function stopGroup(groupId: string): Promise<void> {
  if (!isTauriRuntime()) throw new Error(groupsUnavailableMessage);
  await invoke("stop_group", { groupId });
}

/** Aligns the native title bar with the app theme. `null` lets it follow the operating system. */
export async function setNativeWindowTheme(theme: ResolvedTheme | null): Promise<void> {
  if (!isTauriRuntime()) return;
  await getCurrentWindow().setTheme(theme);
}
