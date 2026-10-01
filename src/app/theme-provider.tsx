import { useCallback, useEffect, useLayoutEffect, useMemo, useState, type ReactNode } from "react";
import { ThemeContext, type ThemeContextValue } from "@/hooks/use-theme";
import { browserStorage } from "@/lib/browser-storage";
import { setNativeWindowTheme } from "@/lib/desktop-api";
import {
  applyTheme,
  oppositeTheme,
  readThemePreference,
  resolveTheme,
  systemDarkQuery,
  writeThemePreference,
  type ThemePreference
} from "@/lib/theme";

function systemDarkMedia(): MediaQueryList | undefined {
  return typeof window.matchMedia === "function" ? window.matchMedia(systemDarkQuery) : undefined;
}

export function ThemeProvider({ children }: { children: ReactNode }) {
  const [preference, setPreferenceState] = useState(() => readThemePreference(browserStorage()));
  const [systemPrefersDark, setSystemPrefersDark] = useState(
    () => systemDarkMedia()?.matches ?? true
  );

  useEffect(() => {
    if (preference !== "system") return;
    const media = systemDarkMedia();
    if (!media) return;
    setSystemPrefersDark(media.matches);
    const listener = (event: MediaQueryListEvent) => setSystemPrefersDark(event.matches);
    media.addEventListener("change", listener);
    return () => media.removeEventListener("change", listener);
  }, [preference]);

  const resolved = resolveTheme(preference, systemPrefersDark);

  useLayoutEffect(() => {
    applyTheme(document.documentElement, resolved);
  }, [resolved]);

  useEffect(() => {
    setNativeWindowTheme(preference === "system" ? null : resolved).catch((error: unknown) => {
      console.warn("The native window theme could not be updated.", error);
    });
  }, [preference, resolved]);

  const setPreference = useCallback((next: ThemePreference) => {
    setPreferenceState(next);
    writeThemePreference(browserStorage(), next);
  }, []);

  const toggle = useCallback(
    () => setPreference(oppositeTheme(resolved)),
    [resolved, setPreference]
  );

  const value = useMemo<ThemeContextValue>(
    () => ({ preference, resolved, setPreference, toggle }),
    [preference, resolved, setPreference, toggle]
  );

  return <ThemeContext.Provider value={value}>{children}</ThemeContext.Provider>;
}
