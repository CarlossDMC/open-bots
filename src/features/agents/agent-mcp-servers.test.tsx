// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { AgentMcpServersSection } from "./agent-mcp-servers";
import { demoAgents } from "@/lib/demo-data";
import type { Agent } from "@/types/domain";

afterEach(cleanup);

describe("AgentMcpServersSection", () => {
  it("adds a trimmed server name and removes it again", async () => {
    const onAgentUpdated = vi.fn<(agent: Agent) => void>();
    const agent: Agent = { ...demoAgents[0], status: "idle", mcpServers: [] };
    const { rerender } = render(
      <AgentMcpServersSection agent={agent} onAgentUpdated={onAgentUpdated} />
    );

    fireEvent.change(screen.getByLabelText("MCP server name"), {
      target: { value: "  claude.ai Atlassian " }
    });
    fireEvent.click(screen.getByRole("button", { name: /Add/ }));
    await waitFor(() => expect(onAgentUpdated).toHaveBeenCalledTimes(1));
    const added = onAgentUpdated.mock.calls[0][0];
    expect(added.mcpServers).toEqual(["claude.ai Atlassian"]);

    rerender(<AgentMcpServersSection agent={added} onAgentUpdated={onAgentUpdated} />);
    fireEvent.click(screen.getByRole("button", { name: "Remove claude.ai Atlassian" }));
    await waitFor(() => expect(onAgentUpdated).toHaveBeenCalledTimes(2));
    expect(onAgentUpdated.mock.calls[1][0].mcpServers).toEqual([]);
  });

  it("locks the selection while the agent is working", () => {
    const agent: Agent = { ...demoAgents[0], status: "working", mcpServers: ["github"] };
    render(<AgentMcpServersSection agent={agent} />);

    expect(screen.getByLabelText<HTMLInputElement>("MCP server name").disabled).toBe(true);
    expect(screen.getByRole<HTMLButtonElement>("button", { name: "Remove github" }).disabled).toBe(
      true
    );
    expect(screen.getByText(/once the current turn ends/)).toBeTruthy();
  });
});
