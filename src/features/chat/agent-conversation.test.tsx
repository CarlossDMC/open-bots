// @vitest-environment jsdom
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { AgentConversation } from "./agent-conversation";
import { useConversation } from "@/hooks/use-conversation";
import { demoAgents, demoProviders } from "@/lib/demo-data";

afterEach(cleanup);

vi.mock("@/hooks/use-conversation", () => ({ useConversation: vi.fn() }));
vi.mock("@/features/agents/agent-avatar", () => ({
  AgentAvatar: ({ seed }: { seed?: string }) => <span data-testid={`avatar-${seed}`} />
}));

describe("AgentConversation", () => {
  beforeEach(() => {
    vi.mocked(useConversation).mockReturnValue({
      messages: [],
      loading: false,
      send: vi.fn().mockResolvedValue(false),
      cancel: vi.fn().mockResolvedValue(undefined)
    });
  });

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

  it("shows the sender avatar and message body for an incoming agent message", () => {
    const recipient = { ...demoAgents[0], status: "idle" as const };
    const sender = demoAgents[1];
    vi.mocked(useConversation).mockReturnValue({
      messages: [
        {
          id: "peer-message",
          agentId: recipient.id,
          sourceAgentId: sender.id,
          role: "system",
          content:
            "Message from Nova. To reply, you must use the agent_message tool.\n\nThe API contract is ready.",
          createdAt: "2026-01-10T11:00:00.000Z"
        }
      ],
      loading: false,
      send: vi.fn().mockResolvedValue(false),
      cancel: vi.fn().mockResolvedValue(undefined)
    });

    render(<AgentConversation agent={recipient} agents={demoAgents} provider={demoProviders[0]} />);

    expect(screen.getByText(/Message from/).textContent).toContain(sender.name);
    expect(screen.getByText("The API contract is ready.")).toBeTruthy();
    expect(screen.getByTestId(`avatar-${sender.id}`)).toBeTruthy();
    expect(screen.queryByText(/must use the agent_message tool/)).toBeNull();
  });
});
