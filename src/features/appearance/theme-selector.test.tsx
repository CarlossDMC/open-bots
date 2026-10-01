// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ThemeSelector } from "./theme-selector";
import { ThemeProvider } from "@/app/theme-provider";
import { themeStorageKey } from "@/lib/theme";

const radio = (name: string) => screen.getByRole("radio", { name });
const renderSelector = () =>
  render(
    <ThemeProvider>
      <ThemeSelector />
    </ThemeProvider>
  );

describe("ThemeSelector", () => {
  beforeEach(() => {
    localStorage.clear();
    document.documentElement.className = "";
    Object.defineProperty(window, "matchMedia", {
      configurable: true,
      writable: true,
      value: vi.fn(() => ({
        matches: false,
        addEventListener: vi.fn(),
        removeEventListener: vi.fn()
      }))
    });
  });
  afterEach(cleanup);

  it("checks the current preference and keeps only it in the tab order", () => {
    renderSelector();
    expect(screen.getByRole("radiogroup", { name: "Theme" })).toBeTruthy();
    expect(radio("Dark").getAttribute("aria-checked")).toBe("true");
    expect(radio("Dark").tabIndex).toBe(0);
    expect(radio("Light").tabIndex).toBe(-1);
  });

  it("selects and persists a clicked option", () => {
    renderSelector();
    fireEvent.click(radio("Light"));
    expect(radio("Light").getAttribute("aria-checked")).toBe("true");
    expect(localStorage.getItem(themeStorageKey)).toBe("light");
  });

  it("moves selection and focus with arrow keys, wrapping at the ends", () => {
    renderSelector();
    radio("Dark").focus();
    fireEvent.keyDown(radio("Dark"), { key: "ArrowRight" });
    expect(radio("System").getAttribute("aria-checked")).toBe("true");
    expect(document.activeElement).toBe(radio("System"));
    fireEvent.keyDown(radio("System"), { key: "ArrowLeft" });
    expect(radio("Dark").getAttribute("aria-checked")).toBe("true");
  });
});
