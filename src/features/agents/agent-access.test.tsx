// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { AgentAccessSection } from "./agent-access";
import { demoAgents } from "@/lib/demo-data";
import { canUseInternet, canWriteWorkspace } from "@/lib/permissions";
import type { Agent } from "@/types/domain";

afterEach(cleanup);

const agent: Agent = { ...demoAgents[0], status: "idle" };

describe("AgentAccessSection", () => {
  it("shows the agent's access and saves a change", async () => {
    const onAgentUpdated = vi.fn();
    render(<AgentAccessSection agent={agent} onAgentUpdated={onAgentUpdated} />);

    const internet = screen.getByRole("checkbox", { name: "Internet access" });
    expect(internet.getAttribute("aria-checked")).toBe("true");
    fireEvent.click(internet);

    await waitFor(() => expect(onAgentUpdated).toHaveBeenCalledOnce());
    const updated = onAgentUpdated.mock.calls[0][0] as Agent;
    expect(canUseInternet(updated.permissions)).toBe(false);
    expect(canWriteWorkspace(updated.permissions)).toBe(true);
  });

  it("locks access while the agent is working", () => {
    render(<AgentAccessSection agent={{ ...agent, status: "working" }} />);

    expect(
      screen.getByRole("checkbox", { name: "Edit files and run commands" }).hasAttribute("disabled")
    ).toBe(true);
    expect(screen.getByText("Access can change once the current turn ends.")).toBeTruthy();
  });
});
