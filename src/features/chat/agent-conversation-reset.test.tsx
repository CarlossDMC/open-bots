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

function mockConversation() {
  const resetSession = vi.fn().mockResolvedValue(true);
  vi.mocked(useConversation).mockReturnValue({
    messages: [],
    loading: false,
    send: vi.fn().mockResolvedValue(false),
    cancel: vi.fn().mockResolvedValue(undefined),
    resetSession
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
});
