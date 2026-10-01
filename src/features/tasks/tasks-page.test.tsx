// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { useTasks } from "@/hooks/use-tasks";
import { demoAgents, demoTasks } from "@/lib/demo-data";
import { TasksPage } from "./tasks-page";

vi.mock("@/hooks/use-tasks", () => ({ useTasks: vi.fn() }));
vi.mock("@/features/agents/agent-avatar", () => ({ AgentAvatar: () => null }));

const create = vi.fn();
const clearCreateError = vi.fn();
const setStatus = vi.fn();

describe("TasksPage", () => {
  beforeEach(() => {
    vi.mocked(useTasks).mockReturnValue({
      tasks: demoTasks,
      available: true,
      loading: false,
      create,
      clearCreateError,
      setStatus
    });
  });
  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
  });

  it("opens the dedicated task dialog", () => {
    render(<TasksPage agents={demoAgents} />);
    fireEvent.click(screen.getByRole("button", { name: "New task" }));

    expect(clearCreateError).toHaveBeenCalledOnce();
    expect(screen.getByRole("dialog", { name: "Create task" })).toBeTruthy();
  });

  it("searches task titles, descriptions, and results", () => {
    render(<TasksPage agents={demoAgents} />);
    fireEvent.change(screen.getByLabelText("Search tasks"), { target: { value: "compact" } });

    expect(screen.getByText("Build inventory table")).toBeTruthy();
    expect(screen.queryByText("Implement inventory API")).toBeNull();
    expect(screen.getByText("1 task")).toBeTruthy();
  });

  it("combines status filtering with search and clears the filters", async () => {
    render(<TasksPage agents={demoAgents} />);
    fireEvent.change(screen.getByLabelText("Search tasks"), { target: { value: "inventory" } });
    fireEvent.click(screen.getByRole("combobox", { name: "Filter by status" }));
    fireEvent.click(await screen.findByRole("option", { name: "Running" }));

    expect(screen.getByText("Implement inventory API")).toBeTruthy();
    expect(screen.queryByText("Build inventory table")).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: "Clear" }));

    await waitFor(() => expect(screen.getByText("3 tasks")).toBeTruthy());
    expect(screen.getByText("Build inventory table")).toBeTruthy();
  });

  it("filters tasks by assigned agent", async () => {
    render(<TasksPage agents={demoAgents} />);
    fireEvent.click(screen.getByRole("combobox", { name: "Filter by agent" }));
    fireEvent.click(await screen.findByRole("option", { name: /Nova/ }));

    expect(screen.getByText("Build inventory table")).toBeTruthy();
    expect(screen.queryByText("Implement inventory API")).toBeNull();
    expect(screen.getByText("1 task")).toBeTruthy();
  });

  it("only offers cancellation for unfinished tasks", () => {
    render(<TasksPage agents={demoAgents} />);

    fireEvent.click(screen.getByRole("button", { name: "Cancel Implement inventory API" }));
    expect(setStatus).toHaveBeenCalledWith(demoTasks[0], "cancelled");
    expect(screen.queryByRole("button", { name: "Cancel Review sync strategies" })).toBeNull();
  });
});
