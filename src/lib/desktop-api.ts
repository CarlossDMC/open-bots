import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  demoAgents,
  demoApprovals,
  demoEvents,
  demoProviders,
  demoUsageReports
} from "@/lib/demo-data";
import type { ResolvedTheme } from "@/lib/theme";
import type {
  Agent,
  AgentMemory,
  ConversationMessage,
  ApprovalDecision,
  ApprovalRequest,
  NewAgentInput,
  NewRoutineInput,
  ProviderModel,
  ProviderSummary,
  ProviderUsageReport,
  Routine,
  RuntimeEvent
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

export async function createAgent(input: NewAgentInput): Promise<Agent> {
  if (!isTauriRuntime()) {
    const timestamp = new Date().toISOString();
    return {
      ...input,
      id: crypto.randomUUID(),
      avatarVariant: "orbital",
      status: "idle",
      permissions: {
        filesystem: "workspace-only",
        shell: "approval-required",
        git: "approval-required",
        network: "restricted",
        browser: "denied"
      },
      createdAt: timestamp,
      updatedAt: timestamp
    };
  }
  return invoke<Agent>("create_agent", { input });
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

/** Aligns the native title bar with the app theme. `null` lets it follow the operating system. */
export async function setNativeWindowTheme(theme: ResolvedTheme | null): Promise<void> {
  if (!isTauriRuntime()) return;
  await getCurrentWindow().setTheme(theme);
}
