// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { useState } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { demoAgents } from "@/lib/demo-data";
import { NewGroupDialog } from "./new-group-dialog";

afterEach(cleanup);

describe("NewGroupDialog", () => {
  it("submits trimmed details with members in the order they were picked", async () => {
    const onSubmit = vi.fn().mockResolvedValue(true);
    render(<DialogHarness onSubmit={onSubmit} />);
    const opener = screen.getByRole("button", { name: "Open group dialog" });
    opener.focus();
    fireEvent.click(opener);

    fireEvent.change(screen.getByLabelText("Name"), { target: { value: "  Release  " } });
    fireEvent.change(screen.getByLabelText("Topic"), { target: { value: " Ship 0.4 " } });
    const create = screen.getByRole("button", { name: "Create group" });
    expect(create.hasAttribute("disabled")).toBe(true);
    fireEvent.click(screen.getByRole("checkbox", { name: demoAgents[1].name }));
    fireEvent.click(screen.getByRole("checkbox", { name: demoAgents[0].name }));
    fireEvent.click(create);

    await waitFor(() =>
      expect(onSubmit).toHaveBeenCalledWith({
        name: "Release",
        topic: "Ship 0.4",
        memberIds: [demoAgents[1].id, demoAgents[0].id]
      })
    );
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    expect(document.activeElement).toBe(opener);
  });

  it("keeps the form and shows the error when creation fails", async () => {
    const onSubmit = vi.fn().mockResolvedValue(false);
    render(
      <NewGroupDialog
        open
        available
        agents={demoAgents}
        error="The group could not be created."
        onClose={vi.fn()}
        onSubmit={onSubmit}
      />
    );
    fireEvent.change(screen.getByLabelText("Name"), { target: { value: "Release" } });
    fireEvent.change(screen.getByLabelText("Topic"), { target: { value: "Ship it" } });
    fireEvent.click(screen.getByRole("checkbox", { name: demoAgents[0].name }));
    fireEvent.click(screen.getByRole("button", { name: "Create group" }));

    await waitFor(() => expect(onSubmit).toHaveBeenCalledOnce());
    expect(screen.getByRole("dialog")).toBeTruthy();
    expect(screen.getByLabelText<HTMLInputElement>("Name").value).toBe("Release");
    expect(screen.getByRole("alert").textContent).toContain("could not be created");
  });

  it("explains that the browser preview cannot create groups", () => {
    render(
      <NewGroupDialog
        open
        available={false}
        agents={demoAgents}
        onClose={vi.fn()}
        onSubmit={vi.fn()}
      />
    );
    expect(screen.getByRole("status").textContent).toBe("Groups require the desktop runtime.");
    expect(screen.getByLabelText("Name").hasAttribute("disabled")).toBe(true);
  });
});

function DialogHarness({ onSubmit }: { onSubmit: NewGroupDialogProps["onSubmit"] }) {
  const [open, setOpen] = useState(false);
  return (
    <>
      <button onClick={() => setOpen(true)}>Open group dialog</button>
      <NewGroupDialog
        open={open}
        available
        agents={demoAgents}
        onClose={() => setOpen(false)}
        onSubmit={onSubmit}
      />
    </>
  );
}

type NewGroupDialogProps = React.ComponentProps<typeof NewGroupDialog>;
