// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { useState } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { Combobox, type ComboboxOption } from "./combobox";

afterEach(cleanup);

const options: ComboboxOption[] = [
  { value: "codex", label: "Codex", description: "OpenAI CLI" },
  { value: "claude", label: "Claude Code", description: "Not installed", disabled: true },
  { value: "mock", label: "Mock Provider" }
];

function Harness({ onChange = vi.fn() }: { onChange?: (value: string) => void }) {
  const [value, setValue] = useState("codex");
  return (
    <>
      <Combobox
        aria-label="Provider"
        value={value}
        options={options}
        onChange={(next) => {
          setValue(next);
          onChange(next);
        }}
      />
      <button type="button">Outside</button>
    </>
  );
}

function openList() {
  const trigger = screen.getByRole("combobox", { name: "Provider" });
  fireEvent.click(trigger);
  return { trigger, search: screen.getByRole("searchbox") };
}

describe("Combobox", () => {
  it("shows the selected label and opens a searchable list with focus in the search", () => {
    render(<Harness />);
    const { trigger, search } = openList();
    expect(trigger.textContent).toBe("Codex");
    expect(trigger.getAttribute("aria-expanded")).toBe("true");
    expect(document.activeElement).toBe(search);
    expect(screen.getByRole("option", { name: /Codex/ }).getAttribute("aria-selected")).toBe(
      "true"
    );

    fireEvent.change(search, { target: { value: "mock" } });
    expect(screen.getAllByRole("option").map((option) => option.textContent)).toEqual([
      "Mock Provider"
    ]);
    fireEvent.change(search, { target: { value: "nothing" } });
    expect(screen.getByText("No matches.")).toBeTruthy();
  });

  it("skips disabled options with the keyboard and selects with Enter", () => {
    const onChange = vi.fn();
    render(<Harness onChange={onChange} />);
    const { trigger, search } = openList();

    fireEvent.keyDown(search, { key: "ArrowDown" });
    fireEvent.keyDown(search, { key: "Enter" });

    expect(onChange).toHaveBeenCalledWith("mock");
    expect(screen.queryByRole("listbox")).toBeNull();
    expect(trigger.textContent).toBe("Mock Provider");
    expect(document.activeElement).toBe(trigger);
  });

  it("ignores clicks on disabled options", () => {
    const onChange = vi.fn();
    render(<Harness onChange={onChange} />);
    openList();
    fireEvent.click(screen.getByRole("option", { name: /Claude Code/ }));
    expect(onChange).not.toHaveBeenCalled();
    expect(screen.getByRole("listbox")).toBeTruthy();
  });

  it("closes on Escape and on outside pointer presses", () => {
    render(<Harness />);
    const { trigger, search } = openList();
    fireEvent.keyDown(search, { key: "Escape" });
    expect(screen.queryByRole("listbox")).toBeNull();
    expect(document.activeElement).toBe(trigger);

    openList();
    fireEvent.pointerDown(screen.getByRole("button", { name: "Outside" }));
    expect(screen.queryByRole("listbox")).toBeNull();
  });

  it("opens from the keyboard and stays closed when disabled", () => {
    const { rerender } = render(<Harness />);
    fireEvent.keyDown(screen.getByRole("combobox"), { key: "ArrowDown" });
    expect(screen.getByRole("listbox")).toBeTruthy();

    rerender(
      <Combobox aria-label="Provider" value="" options={options} onChange={vi.fn()} disabled />
    );
    expect(screen.getByRole("combobox").hasAttribute("disabled")).toBe(true);
  });
});
