// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ThemeProvider } from "./theme-provider";
import { useTheme } from "@/hooks/use-theme";
import { setNativeWindowTheme } from "@/lib/desktop-api";
import { themeStorageKey } from "@/lib/theme";

vi.mock("@/lib/desktop-api", () => ({ setNativeWindowTheme: vi.fn() }));

type Listener = (event: MediaQueryListEvent) => void;

function stubSystemTheme(prefersDark: boolean) {
  const listeners = new Set<Listener>();
  const media = {
    matches: prefersDark,
    addEventListener: vi.fn((_type: string, listener: Listener) => {
      listeners.add(listener);
    }),
    removeEventListener: vi.fn((_type: string, listener: Listener) => {
      listeners.delete(listener);
    })
  };
  Object.defineProperty(window, "matchMedia", {
    configurable: true,
    writable: true,
    value: vi.fn(() => media)
  });
  return {
    media,
    change(next: boolean) {
      media.matches = next;
      act(() =>
        listeners.forEach((listener) => listener({ matches: next } as MediaQueryListEvent))
      );
    }
  };
}

function Probe() {
  const { preference, resolved, setPreference, toggle } = useTheme();
  return (
    <div>
      <p data-testid="preference">{preference}</p>
      <p data-testid="resolved">{resolved}</p>
      <button onClick={() => setPreference("light")}>light</button>
      <button onClick={toggle}>toggle</button>
    </div>
  );
}

const root = document.documentElement;
const text = (id: string) => screen.getByTestId(id).textContent;
const renderProbe = () =>
  render(
    <ThemeProvider>
      <Probe />
    </ThemeProvider>
  );

describe("ThemeProvider", () => {
  beforeEach(() => {
    localStorage.clear();
    root.className = "";
    vi.mocked(setNativeWindowTheme).mockReset().mockResolvedValue(undefined);
  });
  afterEach(() => {
    cleanup();
    vi.restoreAllMocks();
  });

  it("defaults to the dark theme", () => {
    stubSystemTheme(false);
    renderProbe();
    expect(text("preference")).toBe("dark");
    expect(text("resolved")).toBe("dark");
    expect(root.classList.contains("dark")).toBe(true);
  });

  it("restores a stored preference", () => {
    localStorage.setItem(themeStorageKey, "light");
    stubSystemTheme(true);
    renderProbe();
    expect(text("resolved")).toBe("light");
    expect(root.classList.contains("dark")).toBe(false);
  });

  it("follows the operating system while the preference is system", () => {
    localStorage.setItem(themeStorageKey, "system");
    const system = stubSystemTheme(true);
    renderProbe();
    expect(text("resolved")).toBe("dark");
    system.change(false);
    expect(text("resolved")).toBe("light");
    expect(root.classList.contains("dark")).toBe(false);
  });

  it("ignores operating system changes after an explicit choice", () => {
    localStorage.setItem(themeStorageKey, "system");
    const system = stubSystemTheme(true);
    renderProbe();
    fireEvent.click(screen.getByText("light"));
    expect(system.media.removeEventListener).toHaveBeenCalled();
    system.change(true);
    expect(text("resolved")).toBe("light");
  });

  it("persists explicit choices", () => {
    stubSystemTheme(false);
    renderProbe();
    fireEvent.click(screen.getByText("light"));
    expect(localStorage.getItem(themeStorageKey)).toBe("light");
  });

  it("toggle pins the opposite of the current system theme", () => {
    localStorage.setItem(themeStorageKey, "system");
    stubSystemTheme(true);
    renderProbe();
    fireEvent.click(screen.getByText("toggle"));
    expect(text("preference")).toBe("light");
    expect(text("resolved")).toBe("light");
  });

  it("lets the native window follow the system only in system mode", () => {
    localStorage.setItem(themeStorageKey, "system");
    stubSystemTheme(true);
    renderProbe();
    expect(setNativeWindowTheme).toHaveBeenLastCalledWith(null);
    fireEvent.click(screen.getByText("light"));
    expect(setNativeWindowTheme).toHaveBeenLastCalledWith("light");
  });

  it("keeps the UI theme when the native window theme cannot be updated", async () => {
    const warn = vi.spyOn(console, "warn").mockImplementation(() => undefined);
    vi.mocked(setNativeWindowTheme).mockRejectedValue(new Error("Permission denied"));
    stubSystemTheme(false);
    renderProbe();
    await vi.waitFor(() => expect(warn).toHaveBeenCalled());
    expect(text("resolved")).toBe("dark");
    expect(root.classList.contains("dark")).toBe(true);
  });
});
