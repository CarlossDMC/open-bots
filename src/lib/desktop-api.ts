import { invoke } from "@tauri-apps/api/core";
import { demoAgents, demoProviders } from "@/lib/demo-data";
import type { Agent, NewAgentInput, ProviderSummary } from "@/types/domain";

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
