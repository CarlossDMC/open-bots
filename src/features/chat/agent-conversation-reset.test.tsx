// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { AgentConversation } from "./agent-conversation";
import { useConversation } from "@/hooks/use-conversation";
import type * as DesktopApi from "@/lib/desktop-api";
import { demoAgents } from "@/lib/demo-data";
import type { ProviderSummary } from "@/types/domain";

vi.mock("@/hooks/use-conversation", () => ({ useConversation: vi.fn() }));
vi.mock("@/lib/desktop-api", async (importOriginal) => ({
  ...(await importOriginal<typeof DesktopApi>()),
  isTauriRuntime: () => true
}));

afterEach(cleanup);

const provider: ProviderSummary = {
  id: "claude-code",
  name: "Claude Code",
  kind: "cli",
  status: "available",
  detail: "",
  capabilities: []
};

function mockConversation(clear = vi.fn().mockResolvedValue(true)) {
  const resetSession = vi.fn().mockResolvedValue(true);
  vi.mocked(useConversation).mockReturnValue({
    messages: [],
    loading: false,
    send: vi.fn().mockResolvedValue(false),
    cancel: vi.fn().mockResolvedValue(undefined),
    resetSession,
    clear
  });
  return resetSession;
}

describe("AgentConversation session reset", () => {
  it("asks for confirmation before starting a new session", async () => {
    const resetSession = mockConversation();
    const agent = { ...demoAgents[0], status: "idle" as const };
    render(<AgentConversation agent={agent} provider={provider} />);

    fireEvent.click(screen.getByRole("button", { name: "Start a new session" }));
    expect(resetSession).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole("button", { name: "New session" }));

    await waitFor(() => expect(resetSession).toHaveBeenCalledTimes(1));
    expect(await screen.findByRole("button", { name: "Start a new session" })).toBeTruthy();
  });

  it("cannot start a new session while the agent is working", () => {
    mockConversation();
    render(
      <AgentConversation agent={{ ...demoAgents[0], status: "working" }} provider={provider} />
    );

    expect(
      screen.getByRole<HTMLButtonElement>("button", { name: "Start a new session" }).disabled
    ).toBe(true);
  });

  it("asks for confirmation before clearing the conversation", async () => {
    const clear = vi.fn().mockResolvedValue(true);
    mockConversation(clear);
    render(<AgentConversation agent={{ ...demoAgents[0], status: "idle" }} provider={provider} />);

    fireEvent.click(screen.getByRole("button", { name: "Clear conversation" }));
    expect(clear).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole("button", { name: "Clear" }));

    await waitFor(() => expect(clear).toHaveBeenCalledOnce());
  });

  it("deletes the agent only after confirmation and only when offered", async () => {
    mockConversation();
    const agent = { ...demoAgents[0], status: "idle" as const };
    const { rerender } = render(<AgentConversation agent={agent} provider={provider} />);
    expect(screen.queryByRole("button", { name: "Delete agent" })).toBeNull();

    const onDelete = vi.fn().mockResolvedValue(true);
    rerender(<AgentConversation agent={agent} provider={provider} onDelete={onDelete} />);
    fireEvent.click(screen.getByRole("button", { name: "Delete agent" }));
    expect(screen.getByText(`Delete ${agent.name} and its history?`)).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Delete" }));

    await waitFor(() => expect(onDelete).toHaveBeenCalledWith(agent));
  });
});
