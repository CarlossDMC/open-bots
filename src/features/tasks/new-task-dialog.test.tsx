// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { useState } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { demoAgents } from "@/lib/demo-data";
import { NewTaskDialog } from "./new-task-dialog";

afterEach(cleanup);

describe("NewTaskDialog", () => {
  it("submits trimmed task details and closes after success", async () => {
    const onSubmit = vi.fn().mockResolvedValue(true);
    render(<DialogHarness onSubmit={onSubmit} />);

    const opener = screen.getByRole("button", { name: "Open task dialog" });
    opener.focus();
    fireEvent.click(opener);
    fireEvent.change(screen.getByLabelText("Title"), { target: { value: "  Review API  " } });
    fireEvent.change(screen.getByLabelText("Description"), {
      target: { value: "  Check the response contract.  " }
    });

    fireEvent.click(screen.getByRole("combobox", { name: "Assigned agent" }));
    fireEvent.click(await screen.findByRole("option", { name: /Atlas/ }));
    fireEvent.click(screen.getByRole("button", { name: "Create task" }));

    await waitFor(() =>
      expect(onSubmit).toHaveBeenCalledWith({
        title: "Review API",
        description: "Check the response contract.",
        assignedAgentId: demoAgents[0].id
      })
    );
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    expect(document.activeElement).toBe(opener);
  });

  it("keeps entered values visible after creation fails", async () => {
    const onSubmit = vi.fn().mockResolvedValue(false);
    render(
      <NewTaskDialog
        open
        available
        agents={demoAgents}
        error="The task could not be created."
        onClose={vi.fn()}
        onSubmit={onSubmit}
      />
    );

    fireEvent.change(screen.getByLabelText("Title"), { target: { value: "Keep this title" } });
    fireEvent.click(screen.getByRole("button", { name: "Create task" }));

    await waitFor(() => expect(onSubmit).toHaveBeenCalledOnce());
    expect(screen.getByRole("dialog")).toBeTruthy();
    expect(screen.getByLabelText<HTMLInputElement>("Title").value).toBe("Keep this title");
    expect(screen.getByRole("alert").textContent).toContain("could not be created");
  });

  it("closes with Escape and restores focus", async () => {
    render(<DialogHarness onSubmit={vi.fn().mockResolvedValue(true)} />);
    const opener = screen.getByRole("button", { name: "Open task dialog" });
    opener.focus();
    fireEvent.click(opener);
    expect(screen.getByRole("dialog")).toBeTruthy();

    fireEvent.keyDown(screen.getByRole("dialog"), { key: "Escape" });

    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    expect(document.activeElement).toBe(opener);
  });
});

function DialogHarness({ onSubmit }: { onSubmit: NewTaskDialogProps["onSubmit"] }) {
  const [open, setOpen] = useState(false);
  return (
    <>
      <button onClick={() => setOpen(true)}>Open task dialog</button>
      <NewTaskDialog
        open={open}
        available
        agents={demoAgents}
        onClose={() => setOpen(false)}
        onSubmit={onSubmit}
      />
    </>
  );
}

type NewTaskDialogProps = React.ComponentProps<typeof NewTaskDialog>;
