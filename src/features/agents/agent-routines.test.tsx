// @vitest-environment jsdom
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { AgentRoutinesSection } from "./agent-routines";

afterEach(cleanup);

describe("AgentRoutinesSection", () => {
  it("explains that routines need the desktop runtime in the browser preview", async () => {
    render(<AgentRoutinesSection agentId="routine-test-agent" />);

    expect(await screen.findByText("Routines require the desktop runtime.")).toBeTruthy();
    expect(screen.getByRole("button", { name: "Add routine" }).hasAttribute("disabled")).toBe(true);
    expect(screen.getByLabelText("Routine name").hasAttribute("disabled")).toBe(true);
  });
});
