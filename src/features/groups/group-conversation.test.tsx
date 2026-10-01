// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { GroupConversation } from "./group-conversation";
import { useGroupConversation } from "@/hooks/use-group-conversation";
import type * as DesktopApi from "@/lib/desktop-api";
import { demoAgents } from "@/lib/demo-data";
import type { Group } from "@/types/domain";

afterEach(cleanup);

vi.mock("@/hooks/use-group-conversation", () => ({ useGroupConversation: vi.fn() }));
vi.mock("@/lib/desktop-api", async (importOriginal) => ({
  ...(await importOriginal<typeof DesktopApi>()),
  isTauriRuntime: () => true
}));
vi.mock("@/features/agents/agent-avatar", () => ({
  AgentAvatar: ({ seed }: { seed?: string }) => <span data-testid={`avatar-${seed}`} />
}));

const [atlas, nova] = demoAgents;

function group(overrides: Partial<Group> = {}): Group {
  return {
    id: "group-1",
    name: "Release",
    topic: "Ship version 0.4",
    memberIds: [atlas.id, nova.id],
    createdBy: { kind: "user" },
    round: { queue: [] },
    createdAt: "2026-01-10T10:00:00.000Z",
    updatedAt: "2026-01-10T10:00:00.000Z",
    ...overrides
  };
}

describe("GroupConversation", () => {
  const stop = vi.fn().mockResolvedValue(undefined);

  beforeEach(() => {
    vi.mocked(useGroupConversation).mockReturnValue({
      messages: [
        {
          id: "m1",
          groupId: "group-1",
          author: { kind: "user" },
          content: "Status?",
          createdAt: "2026-01-10T10:01:00.000Z"
        },
        {
          id: "m2",
          groupId: "group-1",
          author: { kind: "agent", agentId: nova.id },
          content: "The build is green.",
          createdAt: "2026-01-10T10:02:00.000Z"
        }
      ],
      loading: false,
      send: vi.fn().mockResolvedValue(true),
      stop
    });
  });

  it("shows the topic, members, and who wrote each message", () => {
    render(<GroupConversation group={group()} agents={demoAgents} onDelete={vi.fn()} />);

    expect(screen.getByRole("heading", { name: "Release" })).toBeTruthy();
    expect(screen.getByText("Ship version 0.4")).toBeTruthy();
    const members = screen.getByRole("list", { name: "Members" });
    expect(members.querySelectorAll("li")).toHaveLength(2);
    expect(screen.getByText("The build is green.")).toBeTruthy();
    expect(screen.getByText(nova.name)).toBeTruthy();
    expect(screen.getByText("Status?")).toBeTruthy();
  });

  it("names the member answering and lets the user stop the round", () => {
    render(
      <GroupConversation
        group={group({ round: { queue: [{ agentId: atlas.id, chainDepth: 0 }] } })}
        agents={demoAgents}
        onDelete={vi.fn()}
      />
    );

    expect(screen.getByRole("status").textContent).toContain(`${atlas.name} is working`);
    expect(screen.getByText(`${atlas.name} is answering.`)).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Stop" }));
    expect(stop).toHaveBeenCalledOnce();
  });

  it("asks for confirmation before deleting the group", async () => {
    const onDelete = vi.fn().mockResolvedValue(true);
    const deleted = group();
    render(<GroupConversation group={deleted} agents={demoAgents} onDelete={onDelete} />);

    fireEvent.click(screen.getByRole("button", { name: "Delete group" }));
    expect(onDelete).not.toHaveBeenCalled();
    expect(screen.getByText("Delete this group?")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Delete" }));

    await waitFor(() => expect(onDelete).toHaveBeenCalledWith(deleted));
  });

  it("suggests members after @ and inserts the chosen name", () => {
    render(<GroupConversation group={group()} agents={demoAgents} onDelete={vi.fn()} />);
    const input = screen.getByLabelText<HTMLTextAreaElement>("Message Release");

    fireEvent.change(input, { target: { value: "Hi @n", selectionStart: 5 } });
    const options = screen.getAllByRole("option");
    expect(options.map((option) => option.textContent)).toEqual([`${nova.name}${nova.role}`]);
    fireEvent.keyDown(input, { key: "Enter" });

    expect(input.value).toBe(`Hi @${nova.name} `);
    expect(screen.queryByRole("listbox")).toBeNull();
  });

  it("links mentions in messages to the mentioned member", () => {
    vi.mocked(useGroupConversation).mockReturnValue({
      messages: [
        {
          id: "m3",
          groupId: "group-1",
          author: { kind: "agent", agentId: atlas.id },
          content: `@${nova.name} can you review?`,
          createdAt: "2026-01-10T10:03:00.000Z"
        }
      ],
      loading: false,
      send: vi.fn().mockResolvedValue(true),
      stop
    });
    const onOpenAgent = vi.fn();
    render(
      <GroupConversation
        group={group()}
        agents={demoAgents}
        onDelete={vi.fn()}
        onOpenAgent={onOpenAgent}
      />
    );

    fireEvent.click(screen.getByRole("button", { name: `@${nova.name}` }));
    expect(onOpenAgent).toHaveBeenCalledWith(nova);
  });
});
