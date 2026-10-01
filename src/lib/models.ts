import { providerCapabilities, type Agent, type ProviderSummary } from "@/types/domain";

export function supportsModelSelection(provider?: ProviderSummary): boolean {
  return provider?.capabilities.includes(providerCapabilities.modelSelection) ?? false;
}

/** Short model label such as "gpt-5.5 · high", or the provider default. */
export function describeModel(agent: Pick<Agent, "model" | "reasoningEffort">): string {
  if (!agent.model) return "Provider default";
  return agent.reasoningEffort ? `${agent.model} · ${agent.reasoningEffort}` : agent.model;
}
