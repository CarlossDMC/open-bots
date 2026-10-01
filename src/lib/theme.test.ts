// @vitest-environment jsdom
import { beforeEach, describe, expect, it } from "vitest";
import {
  applyTheme,
  oppositeTheme,
  parseThemePreference,
  readThemePreference,
  resolveTheme,
  themePreferences,
  themeStorageKey,
  themeSwitchingClass,
  writeThemePreference
} from "./theme";

describe("parseThemePreference", () => {
  it("accepts every known preference", () => {
    for (const preference of themePreferences) {
      expect(parseThemePreference(preference)).toBe(preference);
    }
  });

  it.each([null, undefined, "", "Dark", "sepia"])("falls back to dark for %s", (raw) => {
    expect(parseThemePreference(raw)).toBe("dark");
  });
});

describe("resolveTheme", () => {
  it.each([
    ["light", false, "light"],
    ["light", true, "light"],
    ["dark", false, "dark"],
    ["dark", true, "dark"],
    ["system", false, "light"],
    ["system", true, "dark"]
  ] as const)("resolves %s with system dark %s to %s", (preference, systemDark, expected) => {
    expect(resolveTheme(preference, systemDark)).toBe(expected);
  });
});

describe("oppositeTheme", () => {
  it("flips the resolved theme", () => {
    expect(oppositeTheme("dark")).toBe("light");
    expect(oppositeTheme("light")).toBe("dark");
  });
});

describe("theme preference storage", () => {
  it("round-trips a preference", () => {
    const values = new Map<string, string>();
    const storage = {
      getItem: (key: string) => values.get(key) ?? null,
      setItem: (key: string, value: string) => void values.set(key, value)
    };
    writeThemePreference(storage, "light");
    expect(values.get(themeStorageKey)).toBe("light");
    expect(readThemePreference(storage)).toBe("light");
  });

  it("defaults to dark when storage is missing or throws", () => {
    const denied = () => {
      throw new Error("Storage access denied");
    };
    const throwing = { getItem: denied, setItem: denied };
    expect(readThemePreference(undefined)).toBe("dark");
    expect(readThemePreference(throwing)).toBe("dark");
    expect(() => writeThemePreference(throwing, "light")).not.toThrow();
  });
});

describe("applyTheme", () => {
  const root = document.documentElement;
  let frames: (() => void)[] = [];
  const scheduleFrame = (callback: () => void) => {
    frames.push(callback);
  };
  const flushFrames = () => {
    const pending = frames;
    frames = [];
    pending.forEach((callback) => callback());
  };

  beforeEach(() => {
    frames = [];
    document.head.innerHTML =
      '<meta name="theme-color" content="#000000" />' +
      "<style>:root { --background: 0 0% 100%; } .dark { --background: 240 6% 4%; }</style>";
    root.className = "";
  });

  it("applies the class, color scheme, and window color for each theme", () => {
    const meta = () => document.querySelector('meta[name="theme-color"]')?.getAttribute("content");

    applyTheme(root, "dark", scheduleFrame);
    expect(root.classList.contains("dark")).toBe(true);
    expect(root.style.colorScheme).toBe("dark");
    expect(meta()).toBe("hsl(240 6% 4%)");

    applyTheme(root, "light", scheduleFrame);
    expect(root.classList.contains("dark")).toBe(false);
    expect(root.style.colorScheme).toBe("light");
    expect(meta()).toBe("hsl(0 0% 100%)");
  });

  it("suspends transitions until the next frame, even when toggled rapidly", () => {
    applyTheme(root, "light", scheduleFrame);
    applyTheme(root, "dark", scheduleFrame);
    expect(root.classList.contains(themeSwitchingClass)).toBe(true);
    flushFrames();
    expect(root.classList.contains(themeSwitchingClass)).toBe(false);
  });
});
