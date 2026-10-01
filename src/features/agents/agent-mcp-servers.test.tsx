// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { AgentMcpServersSection } from "./agent-mcp-servers";
import { demoAgents } from "@/lib/demo-data";
import type { Agent, McpCatalogEntry, ProviderSummary } from "@/types/domain";

afterEach(cleanup);

const provider: ProviderSummary = {
  id: "claude-code",
  name: "Claude Code",
  kind: "cli",
  status: "available",
  detail: "",
  capabilities: ["configured_mcp_servers"]
};
const entry = (providerId: string, name: string): McpCatalogEntry => ({
  providerId,
  name,
  addedAt: "2026-10-01T12:00:00Z"
});
const entries = [
  entry("claude-code", "claude.ai Atlassian"),
  entry("claude-code", "github"),
  entry("codex", "docs")
];
const agent: Agent = {
  ...demoAgents[0],
  providerId: "claude-code",
  status: "idle",
  mcpServers: ["github", "removed from catalog"]
};

describe("AgentMcpServersSection", () => {
  it("selects servers from the provider's catalog entries", async () => {
    const onAgentUpdated = vi.fn<(agent: Agent) => void>();
    render(
      <AgentMcpServersSection
        agent={agent}
        provider={provider}
        entries={entries}
        onAgentUpdated={onAgentUpdated}
      />
    );

    expect(screen.queryByText("docs")).toBeNull();
    expect(screen.getByText(/1 catalog server belongs to another provider/)).toBeTruthy();
    fireEvent.click(screen.getByRole("checkbox", { name: "claude.ai Atlassian" }));

    await waitFor(() => expect(onAgentUpdated).toHaveBeenCalledTimes(1));
    expect(onAgentUpdated.mock.calls[0][0].mcpServers).toEqual(["github", "claude.ai Atlassian"]);
  });

  it("locks the selection while the agent is working", () => {
    render(
      <AgentMcpServersSection
        agent={{ ...agent, status: "working" }}
        provider={provider}
        entries={entries}
      />
    );

    expect(screen.getByRole<HTMLButtonElement>("checkbox", { name: "github" }).disabled).toBe(true);
    expect(screen.getByText(/once the current turn ends/)).toBeTruthy();
  });
});
