export const themePreferences = ["system", "light", "dark"] as const;
export type ThemePreference = (typeof themePreferences)[number];
export type ResolvedTheme = "light" | "dark";

// Keep the key and fallback in sync with public/theme-init.js, which runs before React loads.
export const themeStorageKey = "open-bots.theme";
export const defaultThemePreference: ThemePreference = "dark";
export const systemDarkQuery = "(prefers-color-scheme: dark)";
export const themeSwitchingClass = "theme-switching";

export function parseThemePreference(raw: string | null | undefined): ThemePreference {
  return themePreferences.find((preference) => preference === raw) ?? defaultThemePreference;
}

export function resolveTheme(
  preference: ThemePreference,
  systemPrefersDark: boolean
): ResolvedTheme {
  if (preference === "system") return systemPrefersDark ? "dark" : "light";
  return preference;
}

export function oppositeTheme(theme: ResolvedTheme): ResolvedTheme {
  return theme === "dark" ? "light" : "dark";
}

export function readThemePreference(
  storage: Pick<Storage, "getItem"> | undefined
): ThemePreference {
  try {
    return parseThemePreference(storage?.getItem(themeStorageKey));
  } catch {
    return defaultThemePreference;
  }
}

export function writeThemePreference(
  storage: Pick<Storage, "setItem"> | undefined,
  preference: ThemePreference
): void {
  try {
    storage?.setItem(themeStorageKey, preference);
  } catch {
    // Storage can be unavailable; the preference then only lasts for this session.
  }
}

function nextFrame(callback: () => void): void {
  if (typeof window.requestAnimationFrame === "function") window.requestAnimationFrame(callback);
  else window.setTimeout(callback, 0);
}

export function applyTheme(
  root: HTMLElement,
  theme: ResolvedTheme,
  scheduleFrame: (callback: () => void) => void = nextFrame
): void {
  // Suspend CSS transitions so every surface switches in the same frame.
  root.classList.add(themeSwitchingClass);
  root.classList.toggle("dark", theme === "dark");
  root.style.colorScheme = theme;
  // Reading computed style also flushes the new theme before transitions resume.
  const background = root.ownerDocument.defaultView
    ?.getComputedStyle(root)
    .getPropertyValue("--background")
    .trim();
  if (background) {
    root.ownerDocument
      .querySelector('meta[name="theme-color"]')
      ?.setAttribute("content", `hsl(${background})`);
  }
  scheduleFrame(() => root.classList.remove(themeSwitchingClass));
}
