// @vitest-environment jsdom
import { readFileSync } from "node:fs";
import path from "node:path";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { systemDarkQuery, themeStorageKey } from "./theme";

const script = readFileSync(path.join(process.cwd(), "public/theme-init.js"), "utf8");
const root = document.documentElement;

function stubSystemDark(prefersDark: boolean) {
  Object.defineProperty(window, "matchMedia", {
    configurable: true,
    writable: true,
    value: (query: string) => ({ matches: query === systemDarkQuery && prefersDark })
  });
}

function runScript() {
  // Indirect eval runs the script in global scope, as the browser does for a classic script.
  globalThis.eval(script);
}

describe("theme-init.js", () => {
  beforeEach(() => {
    localStorage.clear();
    root.className = "";
    stubSystemDark(false);
  });
  afterEach(() => vi.restoreAllMocks());

  it.each([
    [null, false, true],
    ["dark", false, true],
    ["light", true, false],
    ["system", true, true],
    ["system", false, false],
    ["sepia", false, true]
  ] as const)("stored %s with system dark %s applies dark: %s", (stored, systemDark, dark) => {
    if (stored !== null) localStorage.setItem(themeStorageKey, stored);
    stubSystemDark(systemDark);
    runScript();
    expect(root.classList.contains("dark")).toBe(dark);
    expect(root.style.colorScheme).toBe(dark ? "dark" : "light");
  });

  it("falls back to dark when storage throws", () => {
    vi.spyOn(Storage.prototype, "getItem").mockImplementation(() => {
      throw new Error("Storage access denied");
    });
    runScript();
    expect(root.classList.contains("dark")).toBe(true);
  });
});
