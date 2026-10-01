// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { AgentMemorySection } from "./agent-memory";

afterEach(cleanup);

describe("AgentMemorySection", () => {
  it("adds and removes memories in the browser preview", async () => {
    render(<AgentMemorySection agentId="memory-test-agent" />);
    expect(await screen.findByText("No memories yet.")).toBeTruthy();

    const input = screen.getByLabelText("New memory");
    const add = screen.getByRole("button", { name: "Add" });
    expect(add.hasAttribute("disabled")).toBe(true);

    fireEvent.change(input, { target: { value: "  Prefers small commits  " } });
    fireEvent.click(add);

    expect(await screen.findByText("Prefers small commits")).toBeTruthy();
    expect((input as HTMLInputElement).value).toBe("");

    fireEvent.click(screen.getByRole("button", { name: "Remove memory" }));
    expect(await screen.findByText("No memories yet.")).toBeTruthy();
  });
});
