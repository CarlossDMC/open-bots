// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { useState } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { Checkbox } from "./checkbox";

afterEach(cleanup);

function Labelled({ onChange = vi.fn() }: { onChange?: (checked: boolean) => void }) {
  const [checked, setChecked] = useState(false);
  return (
    <label>
      <span>Check on launch</span>
      <Checkbox
        checked={checked}
        onCheckedChange={(next) => {
          setChecked(next);
          onChange(next);
        }}
      />
    </label>
  );
}

describe("Checkbox", () => {
  it("exposes its state and toggles when clicked", () => {
    const onChange = vi.fn();
    render(<Labelled onChange={onChange} />);
    const checkbox = screen.getByRole("checkbox", { name: "Check on launch" });
    expect(checkbox.getAttribute("aria-checked")).toBe("false");

    fireEvent.click(checkbox);
    expect(onChange).toHaveBeenLastCalledWith(true);
    expect(checkbox.getAttribute("aria-checked")).toBe("true");
  });

  it("toggles from the wrapping label text", () => {
    const onChange = vi.fn();
    render(<Labelled onChange={onChange} />);
    fireEvent.click(screen.getByText("Check on launch"));
    expect(onChange).toHaveBeenCalledWith(true);
  });

  it("does not toggle on Enter or while disabled", () => {
    const onChange = vi.fn();
    const { rerender } = render(
      <Checkbox aria-label="Enable" checked={false} onCheckedChange={onChange} />
    );
    const enter = new KeyboardEvent("keydown", { key: "Enter", bubbles: true, cancelable: true });
    screen.getByRole("checkbox").dispatchEvent(enter);
    expect(enter.defaultPrevented).toBe(true);

    rerender(<Checkbox aria-label="Enable" checked={false} onCheckedChange={onChange} disabled />);
    fireEvent.click(screen.getByRole("checkbox"));
    expect(onChange).not.toHaveBeenCalled();
  });
});
