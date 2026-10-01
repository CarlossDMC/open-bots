// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ThemeToggle } from "./theme-toggle";
import { MotionProvider } from "@/app/motion-provider";
import { ThemeProvider } from "@/app/theme-provider";

function stubMediaQueries({ reducedMotion }: { reducedMotion: boolean }) {
  Object.defineProperty(window, "matchMedia", {
    configurable: true,
    writable: true,
    value: vi.fn((query: string) => ({
      matches: query.includes("prefers-reduced-motion") && reducedMotion,
      media: query,
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
      addListener: vi.fn(),
      removeListener: vi.fn()
    }))
  });
}

const renderToggle = () =>
  render(
    <ThemeProvider>
      <MotionProvider>
        <ThemeToggle />
      </MotionProvider>
    </ThemeProvider>
  );

describe("ThemeToggle", () => {
  beforeEach(() => {
    localStorage.clear();
    document.documentElement.className = "";
  });
  afterEach(cleanup);

  it.each([false, true])("switches theme with reduced motion %s", (reducedMotion) => {
    stubMediaQueries({ reducedMotion });
    renderToggle();
    fireEvent.click(screen.getByRole("button", { name: "Switch to light theme" }));
    expect(document.documentElement.classList.contains("dark")).toBe(false);
    fireEvent.click(screen.getByRole("button", { name: "Switch to dark theme" }));
    expect(document.documentElement.classList.contains("dark")).toBe(true);
  });
});
