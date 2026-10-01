// @vitest-environment jsdom
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { AgentConversation } from "./agent-conversation";
import { demoAgents, demoProviders } from "@/lib/demo-data";

afterEach(cleanup);

vi.mock("@/features/agents/agent-avatar", () => ({ AgentAvatar: () => null }));

describe("AgentConversation", () => {
  it("disables the composer in the browser preview and says why", async () => {
    const agent = { ...demoAgents[0], status: "idle" as const };
    render(<AgentConversation agent={agent} provider={demoProviders[0]} />);

    expect(await screen.findByText("Messaging requires the desktop runtime.")).toBeTruthy();
    expect(screen.getByLabelText(`Message ${agent.name}`).hasAttribute("disabled")).toBe(true);
    expect(screen.getByRole("button", { name: "Send" }).hasAttribute("disabled")).toBe(true);
  });

  it("offers a stop control and a working indicator while the agent is working", () => {
    render(<AgentConversation agent={{ ...demoAgents[0], status: "working" }} />);

    expect(screen.getByRole("button", { name: "Stop" })).toBeTruthy();
    expect(screen.getByRole("status").textContent).toContain("is working");
    expect(screen.getByTestId("working-dots")).toBeTruthy();
  });

  it("removes the working indicator once the turn ends", async () => {
    const agent = demoAgents[0];
    const { rerender } = render(<AgentConversation agent={{ ...agent, status: "working" }} />);
    expect(screen.getByRole("status")).toBeTruthy();

    rerender(<AgentConversation agent={{ ...agent, status: "idle" }} />);

    await waitFor(() => expect(screen.queryByRole("status")).toBeNull());
    expect(await screen.findByRole("button", { name: "Send" })).toBeTruthy();
  });
});
