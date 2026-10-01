# Light/Dark Theme, Design Tokens, and Motion Foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add System / Light / Dark themes (default Dark), make every visual value come from one token file so the design system is easy to swap, and introduce Motion (`motion/react`) with shared conventions, starting with an animated theme toggle.

**Architecture:** `src/styles/tokens.css` is the single source of colors, fonts, sizes, radius, and shadows (`:root` = light, `.dark` = dark). Tailwind maps tokens to utilities; components use only token utilities. Pure theme logic lives in `src/lib/theme.ts`; `ThemeProvider` owns state, follows the OS in `system` mode, persists to `localStorage`, and syncs the native title bar through `src/lib/desktop-api.ts`. A classic script in `public/theme-init.js` applies the theme before first paint. Motion timing lives in `src/styles/motion-tokens.ts`, shared by Tailwind and `src/lib/motion.ts` presets; `MotionProvider` wraps the app in `LazyMotion strict` + `MotionConfig reducedMotion="user"`.

**Tech Stack:** React 19, TypeScript 5.8, Tailwind CSS 3.4, Vite 7, Vitest 4 + jsdom + Testing Library, `motion` 13.4.6, Tauri 2 (`@tauri-apps/api/window`).

**Spec:** No separate spec file (the user waived it). The design was agreed in conversation and is summarized in [Design Summary](#design-summary) below; executors treat that section as the spec.

## Global Constraints

- All repository content in English (identifiers, comments, UI copy, docs, test names).
- **Do not commit.** The user asked for no commits. Each task ends with a `git status` checkpoint instead.
- Theme preferences are exactly `"system" | "light" | "dark"`. Default and fallback: `"dark"`.
- Storage key: `open-bots.theme` (in `localStorage`, all access wrapped in `try/catch`).
- Package `motion` `^13.4.6`; import only from `motion/react`. Inside `LazyMotion strict` use `m.*`, never `motion.*`.
- Motion durations: `fast` 120ms, `base` 180ms, `slow` 240ms (ceiling). Easing `standard` `[0.2, 0, 0, 1]`, `exit` `[0.4, 0, 1, 1]`. Animate only `opacity` and transforms.
- After Task 3, no raw Tailwind palette colors (`zinc-*`, `red-*`, `white`, `black`, …), arbitrary colors (`bg-[#…]`), or arbitrary font sizes (`text-[10px]`) outside `src/styles/tokens.css`.
- Status colors (`status-*`) and agent identity colors (`identity-*`) are separate token groups and must never be reused for each other.
- Icons: Lucide only.
- Native window calls go through `src/lib/desktop-api.ts`. No SQL, Rust code, or provider changes; the only Tauri change is one capability permission.
- Prettier: double quotes, semicolons, no trailing commas, print width 100.
- Tests must be deterministic: no network, no real Tauri runtime.

## Review Focus

1. A stale or invalid stored value (`""`, `"Dark"`, `"sepia"`) must yield the dark theme, both in React and in the pre-paint script. Tests: Task 1 Step 1 and Task 4 Step 1.
2. Toggling rapidly must never leave `.theme-switching` on `<html>`; otherwise transitions stay disabled app-wide. Test: Task 1 Step 1 (`applyTheme` rapid-toggle case).
3. An OS theme change after the user picked Light or Dark explicitly must be ignored. Test: Task 6 Step 1.
4. A failing native `setTheme` (missing permission, unsupported platform) must only log a warning; the UI theme still applies. Test: Task 6 Step 1.
5. With `prefers-reduced-motion: reduce`, the toggle must still switch themes; only movement is reduced. Test: Task 7 Step 1.

---

## Design Summary

**Tokens (`src/styles/tokens.css`).** Colors are HSL channels (`240 6% 4%`) so Tailwind utilities support opacity (`bg-card/35`).

| Group             | Tokens                                                                                                         |
| ----------------- | -------------------------------------------------------------------------------------------------------------- |
| Surfaces          | `background`, `surface` (sidebar), `card`, `muted`, `accent` (hover/selected), `overlay` (+ `overlay-opacity`) |
| Borders           | `border`, `border-subtle`, `border-strong`, `ring`, `selection`                                                |
| Text              | `foreground`, `foreground-secondary`, `foreground-muted`, `foreground-subtle`, `foreground-faint`              |
| Primary           | `primary`, `primary-foreground`                                                                                |
| Runtime status    | `status-running`, `status-waiting`, `status-queued`, `status-success`, `status-danger`, `status-neutral`       |
| Feedback surfaces | `danger`, `danger-foreground`, `danger-muted`, `danger-border`, same four for `warning`                        |
| Agent identity    | `identity-indigo`, `-cyan`, `-emerald`, `-amber`, `-rose`, `-violet`                                           |
| Typography        | `font-sans`, `font-mono`, `text-3xs` (9px), `text-2xs` (10px), `text-xs-plus` (11px)                           |
| Shape / elevation | `radius-sm/md/lg/xl`, `shadow-panel`                                                                           |

The light theme uses darker status and identity tones (600-level) for contrast on white.

**Theme behavior.**

- The toggle in the sidebar footer switches between light and dark. In `system` mode, it pins the opposite of the current resolved theme.
- _Settings → Appearance_ has a System / Light / Dark radio group.
- The command palette has a "Toggle Theme" command.
- During a switch, `.theme-switching` disables CSS transitions for one frame, so every surface changes together.
- The native title bar follows the app theme via `getCurrentWindow().setTheme()`, with `null` in system mode. Per the Tauri docs, the theme is app-wide on Linux/macOS. Linux behavior depends on GTK/WM and is not promised.

**Motion conventions.**

- Use the shared presets `fadeUp`, `scaleIn`, `iconSwap`.
- Animate only opacity and transforms, for at most 240ms.
- Animate only to show cause and effect of a user action. No entrance animations on re-render, no animation that implies an inactive runtime capability is active.
- The existing `animate-fade-in` CSS class stays until each component is touched for another reason.

## File Structure

| File                                                                                                                                      | Responsibility                                                                  |
| ----------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------- |
| Create `src/lib/browser-storage.ts`                                                                                                       | Safe `window.localStorage` accessor (moved out of `use-app-updater.ts`).        |
| Create `src/lib/theme.ts`                                                                                                                 | Pure theme types, parsing, resolution, storage, DOM application.                |
| Create `src/lib/theme.test.ts`                                                                                                            | Tests for `theme.ts`.                                                           |
| Create `src/styles/tokens.css`                                                                                                            | **The design system.** Every color, font, size, radius, shadow for both themes. |
| Create `src/styles/motion-tokens.ts`                                                                                                      | Motion durations and easings shared by Tailwind and Motion.                     |
| Create `src/styles/design-tokens.test.ts`                                                                                                 | Token parity between themes; no raw palette colors in components.               |
| Modify `tailwind.config.ts`                                                                                                               | Map tokens to utilities.                                                        |
| Modify `src/index.css`                                                                                                                    | Import tokens, token-based base and component classes, `.theme-switching`.      |
| Modify 15 component files (Task 3)                                                                                                        | Replace raw palette classes with token utilities.                               |
| Create `public/theme-init.js`                                                                                                             | Pre-paint theme application.                                                    |
| Create `src/lib/theme-init.test.ts`                                                                                                       | Executes `theme-init.js` against jsdom.                                         |
| Modify `index.html`, `eslint.config.js`                                                                                                   | Load pre-paint script; browser globals for `public/**/*.js`.                    |
| Create `src/lib/motion.ts` (+ test)                                                                                                       | Motion transitions and variant presets.                                         |
| Create `src/app/motion-provider.tsx`                                                                                                      | `LazyMotion strict` + `MotionConfig reducedMotion="user"`.                      |
| Create `src/hooks/use-theme.ts`                                                                                                           | `ThemeContext` and `useTheme()`.                                                |
| Create `src/app/theme-provider.tsx` (+ test)                                                                                              | Theme state, OS listener, persistence, DOM and native sync.                     |
| Modify `src/lib/desktop-api.ts`                                                                                                           | `setNativeWindowTheme()`.                                                       |
| Modify `src-tauri/capabilities/default.json`                                                                                              | `core:window:allow-set-theme`.                                                  |
| Create `src/features/appearance/theme-toggle.tsx` (+ test)                                                                                | Animated sun/moon toggle.                                                       |
| Create `src/features/appearance/theme-selector.tsx` (+ test)                                                                              | System / Light / Dark radio group.                                              |
| Modify `src/main.tsx`, `src/app/app.tsx`, `src/app/sidebar.tsx`, `src/app/command-palette.tsx`, `src/features/settings/settings-page.tsx` | Wire providers and UI.                                                          |
| Create `docs/adr/0005-design-tokens-and-motion.md`, `docs/design-system.md`                                                               | Decision record and how-to-change guide.                                        |
| Modify `docs/dependency-audit.md`, `docs/architecture.md`                                                                                 | Audit entry; link to the design system.                                         |

---

### Task 1: Theme core logic and shared storage helper

**Files:**

- Create: `src/lib/browser-storage.ts`
- Modify: `src/hooks/use-app-updater.ts:12-18` (remove local `browserStorage`, import shared one)
- Create: `src/lib/theme.ts`
- Test: `src/lib/theme.test.ts`

**Interfaces:**

- Consumes: nothing.
- Produces:
  - `browserStorage(): Storage | undefined`
  - `themePreferences: readonly ["system", "light", "dark"]`
  - `type ThemePreference = "system" | "light" | "dark"`
  - `type ResolvedTheme = "light" | "dark"`
  - `themeStorageKey = "open-bots.theme"`
  - `defaultThemePreference: ThemePreference = "dark"`
  - `systemDarkQuery = "(prefers-color-scheme: dark)"`
  - `themeSwitchingClass = "theme-switching"`
  - `parseThemePreference(raw: string | null | undefined): ThemePreference`
  - `resolveTheme(preference: ThemePreference, systemPrefersDark: boolean): ResolvedTheme`
  - `oppositeTheme(theme: ResolvedTheme): ResolvedTheme`
  - `readThemePreference(storage: Pick<Storage, "getItem"> | undefined): ThemePreference`
  - `writeThemePreference(storage: Pick<Storage, "setItem"> | undefined, preference: ThemePreference): void`
  - `applyTheme(root: HTMLElement, theme: ResolvedTheme, scheduleFrame?: (callback: () => void) => void): void`

- [ ] **Step 1: Write the failing test** — create `src/lib/theme.test.ts`:

```ts
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
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npx vitest run src/lib/theme.test.ts`
Expected: FAIL — `Failed to resolve import "./theme"`.

- [ ] **Step 3: Create `src/lib/browser-storage.ts`**

```ts
export function browserStorage(): Storage | undefined {
  try {
    return window.localStorage;
  } catch {
    return undefined;
  }
}
```

In `src/hooks/use-app-updater.ts`, delete the local `function browserStorage()` (lines 12-18) and add `import { browserStorage } from "@/lib/browser-storage";` with the other `@/lib` imports.

- [ ] **Step 4: Write `src/lib/theme.ts`**

```ts
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
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `npx vitest run src/lib/theme.test.ts src/lib/updater.test.ts`
Expected: PASS (all).

- [ ] **Step 6: Typecheck and lint**

Run: `npm run typecheck && npx eslint src/lib src/hooks --max-warnings 0`
Expected: no errors.

- [ ] **Step 7: Checkpoint (no commit)**

Run: `git status --short`
Expected: only `src/lib/browser-storage.ts`, `src/lib/theme.ts`, `src/lib/theme.test.ts`, `src/hooks/use-app-updater.ts` (plus this plan file).

---

### Task 2: Design tokens as the single source

**Files:**

- Create: `src/styles/tokens.css`
- Create: `src/styles/motion-tokens.ts`
- Modify: `tailwind.config.ts` (full rewrite below)
- Modify: `src/index.css` (full rewrite below)
- Test: `src/styles/design-tokens.test.ts` (parity checks; Task 3 adds the palette scan)

**Interfaces:**

- Consumes: nothing.
- Produces:
  - Tailwind utilities: `bg-/text-/border-` + `background`, `surface`, `card`, `muted`, `accent`, `overlay`, `ring`, `selection`, `border` (`DEFAULT`, `subtle`, `strong`), `foreground` (`DEFAULT`, `secondary`, `muted`, `subtle`, `faint`), `primary` (`DEFAULT`, `foreground`), `status-{running,waiting,queued,success,danger,neutral}`, `danger` / `warning` (`DEFAULT`, `foreground`, `muted`, `border`), `identity-{indigo,cyan,emerald,amber,rose,violet}`.
  - Font sizes `text-3xs`, `text-2xs`, `text-xs-plus`.
  - Durations `duration-fast/base/slow`; easings `ease-standard/exit`.
  - From `src/styles/motion-tokens.ts`: `type CubicBezier = [number, number, number, number]`, `durations: { fast: 120; base: 180; slow: 240 }` (ms), `maxDuration: 240`, `easings: { standard: CubicBezier; exit: CubicBezier }`, `cubicBezier(curve: CubicBezier): string`.

- [ ] **Step 1: Write the failing test** — create `src/styles/design-tokens.test.ts`:

```ts
import { readFileSync } from "node:fs";
import path from "node:path";
import { describe, expect, it } from "vitest";
import tailwindConfig from "../../tailwind.config";

const tokensCss = readFileSync(path.join(process.cwd(), "src/styles/tokens.css"), "utf8");

function declaredTokens(selector: ":root" | ".dark"): Set<string> {
  const escaped = selector.replace(".", "\\.");
  const block = new RegExp(`(?:^|\\n)${escaped}\\s*\\{([^}]*)\\}`).exec(tokensCss)?.[1] ?? "";
  return new Set([...block.matchAll(/(--[a-z0-9-]+)\s*:/g)].map((match) => match[1]));
}

function referencedTokens(pattern: RegExp): string[] {
  const theme = JSON.stringify(tailwindConfig.theme);
  return [...new Set([...theme.matchAll(pattern)].map((match) => match[1]))];
}

describe("design tokens", () => {
  const light = declaredTokens(":root");
  const dark = declaredTokens(".dark");

  it("declares every token Tailwind references", () => {
    const missing = referencedTokens(/var\((--[a-z0-9-]+)\)/g).filter((token) => !light.has(token));
    expect(missing).toEqual([]);
  });

  it("gives every color token a dark theme value", () => {
    const colors = referencedTokens(/hsl\(var\((--[a-z0-9-]+)\)/g);
    expect(colors.length).toBeGreaterThan(30);
    expect(colors.filter((token) => !dark.has(token))).toEqual([]);
  });

  it("does not declare dark-only tokens", () => {
    expect([...dark].filter((token) => !light.has(token))).toEqual([]);
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npx vitest run src/styles/design-tokens.test.ts`
Expected: FAIL — `ENOENT ... src/styles/tokens.css`.

- [ ] **Step 3: Create `src/styles/motion-tokens.ts`**

```ts
// Motion timing shared by Tailwind utilities (tailwind.config.ts) and Motion presets (src/lib/motion.ts).
export type CubicBezier = [number, number, number, number];

/** Durations in milliseconds. `slow` is the ceiling for any UI animation. */
export const durations = { fast: 120, base: 180, slow: 240 } as const;
export const maxDuration = durations.slow;

export const easings: { standard: CubicBezier; exit: CubicBezier } = {
  standard: [0.2, 0, 0, 1],
  exit: [0.4, 0, 1, 1]
};

export function cubicBezier([x1, y1, x2, y2]: CubicBezier): string {
  return `cubic-bezier(${x1}, ${y1}, ${x2}, ${y2})`;
}
```

- [ ] **Step 4: Create `src/styles/tokens.css`**

```css
/*
 * Open Bots design tokens: the single source for colors, typography, shape, and elevation.
 * Change the design system here. Components consume these values only through the Tailwind
 * utilities declared in tailwind.config.ts. Colors are HSL channels so utilities can apply
 * opacity, for example bg-card/35. `:root` is the light theme; `.dark` overrides it.
 */
:root {
  color-scheme: light;

  --font-sans: "Inter", ui-sans-serif, system-ui, sans-serif;
  --font-mono: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
  --text-3xs: 9px;
  --text-2xs: 10px;
  --text-xs-plus: 11px;

  --radius-sm: 0.25rem;
  --radius-md: 0.375rem;
  --radius-lg: 0.5rem;
  --radius-xl: 0.75rem;
  --shadow-panel: 0 14px 40px rgb(0 0 0 / 0.08);

  --background: 0 0% 100%;
  --surface: 240 5% 97%;
  --card: 0 0% 100%;
  --muted: 240 5% 96%;
  --accent: 240 5% 91%;
  --overlay: 240 10% 4%;
  --overlay-opacity: 0.25;

  --border: 240 6% 88%;
  --border-subtle: 240 5% 93%;
  --border-strong: 240 5% 78%;
  --ring: 240 5% 50%;
  --selection: 243 75% 59%;

  --foreground: 240 10% 4%;
  --foreground-secondary: 240 4% 20%;
  --foreground-muted: 240 4% 32%;
  --foreground-subtle: 240 4% 44%;
  --foreground-faint: 240 4% 58%;

  --primary: 240 6% 10%;
  --primary-foreground: 0 0% 98%;

  --status-running: 221 83% 53%;
  --status-waiting: 32 95% 44%;
  --status-queued: 262 83% 58%;
  --status-success: 161 94% 30%;
  --status-danger: 0 72% 51%;
  --status-neutral: 240 5% 65%;

  --danger: 0 72% 51%;
  --danger-foreground: 0 74% 42%;
  --danger-muted: 0 86% 97%;
  --danger-border: 0 96% 89%;
  --warning: 32 95% 44%;
  --warning-foreground: 23 83% 31%;
  --warning-muted: 48 100% 96%;
  --warning-border: 48 97% 77%;

  --identity-indigo: 243 75% 59%;
  --identity-cyan: 192 91% 36%;
  --identity-emerald: 161 94% 30%;
  --identity-amber: 32 95% 44%;
  --identity-rose: 347 77% 50%;
  --identity-violet: 262 83% 58%;
}

.dark {
  color-scheme: dark;

  --shadow-panel: 0 14px 40px rgb(0 0 0 / 0.22);

  --background: 240 6% 4%;
  --surface: 240 8% 5%;
  --card: 240 10% 4%;
  --muted: 240 6% 10%;
  --accent: 240 4% 16%;
  --overlay: 0 0% 0%;
  --overlay-opacity: 0.6;

  --border: 240 4% 16%;
  --border-subtle: 240 6% 10%;
  --border-strong: 240 5% 26%;
  --ring: 240 4% 46%;
  --selection: 234 89% 74%;

  --foreground: 240 5% 94%;
  --foreground-secondary: 240 5% 84%;
  --foreground-muted: 240 5% 65%;
  --foreground-subtle: 240 4% 46%;
  --foreground-faint: 240 5% 34%;

  --primary: 240 5% 96%;
  --primary-foreground: 240 10% 4%;

  --status-running: 213 94% 68%;
  --status-waiting: 43 96% 56%;
  --status-queued: 255 92% 76%;
  --status-success: 158 64% 52%;
  --status-danger: 0 91% 71%;
  --status-neutral: 240 4% 46%;

  --danger: 0 91% 71%;
  --danger-foreground: 0 94% 82%;
  --danger-muted: 0 45% 7%;
  --danger-border: 0 50% 22%;
  --warning: 38 92% 50%;
  --warning-foreground: 46 97% 65%;
  --warning-muted: 30 50% 6%;
  --warning-border: 25 55% 18%;

  --identity-indigo: 234 89% 74%;
  --identity-cyan: 188 86% 53%;
  --identity-emerald: 158 64% 52%;
  --identity-amber: 43 96% 56%;
  --identity-rose: 351 95% 71%;
  --identity-violet: 255 92% 76%;
}
```

- [ ] **Step 5: Rewrite `tailwind.config.ts`**

```ts
import type { Config } from "tailwindcss";
import { cubicBezier, durations, easings } from "./src/styles/motion-tokens";

// Every visual value resolves to a CSS custom property declared in src/styles/tokens.css.
const color = (token: string) => `hsl(var(--${token}) / <alpha-value>)`;
const ms = (value: number) => `${value}ms`;

export default {
  darkMode: ["class"],
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  theme: {
    extend: {
      colors: {
        background: color("background"),
        surface: color("surface"),
        card: color("card"),
        muted: color("muted"),
        accent: color("accent"),
        overlay: "hsl(var(--overlay) / var(--overlay-opacity))",
        ring: color("ring"),
        selection: color("selection"),
        border: {
          DEFAULT: color("border"),
          subtle: color("border-subtle"),
          strong: color("border-strong")
        },
        foreground: {
          DEFAULT: color("foreground"),
          secondary: color("foreground-secondary"),
          muted: color("foreground-muted"),
          subtle: color("foreground-subtle"),
          faint: color("foreground-faint")
        },
        primary: { DEFAULT: color("primary"), foreground: color("primary-foreground") },
        status: {
          running: color("status-running"),
          waiting: color("status-waiting"),
          queued: color("status-queued"),
          success: color("status-success"),
          danger: color("status-danger"),
          neutral: color("status-neutral")
        },
        danger: {
          DEFAULT: color("danger"),
          foreground: color("danger-foreground"),
          muted: color("danger-muted"),
          border: color("danger-border")
        },
        warning: {
          DEFAULT: color("warning"),
          foreground: color("warning-foreground"),
          muted: color("warning-muted"),
          border: color("warning-border")
        },
        identity: {
          indigo: color("identity-indigo"),
          cyan: color("identity-cyan"),
          emerald: color("identity-emerald"),
          amber: color("identity-amber"),
          rose: color("identity-rose"),
          violet: color("identity-violet")
        }
      },
      fontFamily: { sans: ["var(--font-sans)"], mono: ["var(--font-mono)"] },
      fontSize: {
        "3xs": "var(--text-3xs)",
        "2xs": "var(--text-2xs)",
        "xs-plus": "var(--text-xs-plus)"
      },
      borderRadius: {
        DEFAULT: "var(--radius-sm)",
        md: "var(--radius-md)",
        lg: "var(--radius-lg)",
        xl: "var(--radius-xl)"
      },
      boxShadow: { panel: "var(--shadow-panel)" },
      transitionDuration: {
        fast: ms(durations.fast),
        base: ms(durations.base),
        slow: ms(durations.slow)
      },
      transitionTimingFunction: {
        standard: cubicBezier(easings.standard),
        exit: cubicBezier(easings.exit)
      },
      keyframes: {
        "fade-in": {
          from: { opacity: "0", transform: "translateY(4px)" },
          to: { opacity: "1", transform: "translateY(0)" }
        }
      },
      animation: { "fade-in": `fade-in ${ms(durations.base)} ${cubicBezier(easings.standard)}` }
    }
  },
  plugins: []
} satisfies Config;
```

- [ ] **Step 6: Rewrite `src/index.css`**

```css
@import "./styles/tokens.css";

@tailwind base;
@tailwind components;
@tailwind utilities;

@layer base {
  * {
    @apply border-border;
  }
  html,
  body,
  #root {
    @apply min-h-full;
  }
  body {
    @apply m-0 bg-background font-sans text-foreground antialiased;
  }
  button,
  input,
  textarea,
  select {
    font: inherit;
  }
  ::selection {
    @apply bg-selection/25 text-foreground;
  }
}

@layer components {
  .panel {
    @apply rounded-lg border border-border/80 bg-card/35 p-4;
  }
  .section-title {
    @apply mb-4 flex items-center gap-2 text-xs font-medium uppercase tracking-wider text-foreground-subtle;
  }
  .label {
    @apply text-xs-plus font-medium uppercase tracking-wider text-foreground-faint;
  }
  .detail-grid {
    @apply grid grid-cols-2 gap-x-8 gap-y-5;
  }
  .field-textarea,
  .field-select {
    @apply w-full rounded-md border border-border bg-card px-3 py-2 text-sm text-foreground outline-none placeholder:text-foreground-faint focus:border-border-strong focus:ring-1 focus:ring-ring/40;
  }
  .field-select {
    @apply h-9 py-0;
  }
  .table-shell {
    @apply overflow-hidden rounded-lg border border-border/80;
  }
  .table-header,
  .table-row {
    @apply grid items-center gap-4 border-b border-border-subtle px-4;
  }
  .table-header {
    @apply bg-card/70 py-2.5 text-2xs font-medium uppercase tracking-wider text-foreground-faint;
  }
  .table-row {
    @apply min-h-16 bg-card/30 py-3 last:border-0 hover:bg-muted/40;
  }
}

/* Set by applyTheme for one frame so a theme change does not cascade through transitions.
   Kept outside @layer so Tailwind never purges it. */
.theme-switching,
.theme-switching * {
  transition: none !important;
}
```

- [ ] **Step 7: Run tests and build**

Run: `npx vitest run src/styles/design-tokens.test.ts && npm run typecheck && npm run build`
Expected: tests PASS; typecheck clean; Vite build succeeds. The UI is not migrated yet, so the remaining `zinc-*` classes still compile because they are standard Tailwind palette classes.

- [ ] **Step 8: Checkpoint (no commit)**

Run: `git status --short`
Expected: Task 1 files, plus `src/styles/`, `tailwind.config.ts`, `src/index.css`.

---

### Task 3: Migrate components to token utilities

**Files:**

- Modify: `src/components/ui/button.tsx`, `src/components/ui/input.tsx`, `src/components/ui/status-badge.tsx`
- Modify: `src/features/agents/agent-avatar.tsx`, `agents-page.tsx`, `agent-details.tsx`, `new-agent-dialog.tsx`
- Modify: `src/features/activity/activity-page.tsx`, `src/features/tasks/tasks-page.tsx`, `src/features/approvals/approvals-page.tsx`, `src/features/settings/settings-page.tsx`, `src/features/updates/update-banner.tsx`, `src/features/updates/update-settings.tsx`
- Modify: `src/app/app.tsx`, `src/app/sidebar.tsx`, `src/app/command-palette.tsx`
- Test: `src/styles/design-tokens.test.ts` (add palette scan)

**Interfaces:**

- Consumes: token utilities from Task 2.
- Produces: no new exports. After this task, components contain only token utilities.

- [ ] **Step 1: Add the failing scan test** — append to `src/styles/design-tokens.test.ts` (merge the new `readdirSync` into the existing `node:fs` import):

```ts
import { readdirSync } from "node:fs";

const sourceRoot = path.join(process.cwd(), "src");
const palette =
  "slate|gray|zinc|neutral|stone|red|orange|amber|yellow|lime|green|emerald|teal|cyan|sky|blue|indigo|violet|purple|fuchsia|pink|rose|white|black";
const rawPaletteClass = new RegExp(
  `(?<![\\w-])(?:[a-z-]+:)*(?:bg|text|border|ring|from|via|to|fill|stroke|accent|outline|divide|placeholder|decoration|shadow)-(?:${palette})(?:-\\d{2,3})?(?:\\/\\d{1,3})?(?![\\w-])`,
  "g"
);
const arbitraryColor = /(?:bg|text|border|ring|from|via|to)-\[(?:#|rgb|hsl)[^\]]*\]/g;
const arbitraryFontSize = /text-\[\d+px\]/g;

describe("component styling", () => {
  const files = readdirSync(sourceRoot, { recursive: true, encoding: "utf8" }).filter(
    (file) =>
      /\.(?:tsx?|css)$/.test(file) &&
      !/\.test\.tsx?$/.test(file) &&
      file !== path.join("styles", "tokens.css")
  );

  it("uses design tokens instead of raw palette values", () => {
    const violations = files.flatMap((file) => {
      const source = readFileSync(path.join(sourceRoot, file), "utf8");
      return [rawPaletteClass, arbitraryColor, arbitraryFontSize].flatMap((pattern) =>
        [...source.matchAll(pattern)].map((match) => `${file}: ${match[0]}`)
      );
    });
    expect(violations).toEqual([]);
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npx vitest run src/styles/design-tokens.test.ts`
Expected: FAIL — the violation list includes about 110 entries such as `app/sidebar.tsx: bg-[#0b0b0d]` and `components/ui/button.tsx: bg-zinc-100`. Keep this list open as the work queue.

- [ ] **Step 3: Rewrite the shared primitives exactly as follows**

`src/components/ui/button.tsx`, the `buttonVariants` definition:

```ts
const buttonVariants = cva(
  "inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md text-sm font-medium transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:pointer-events-none disabled:opacity-50",
  {
    variants: {
      variant: {
        default: "bg-primary text-primary-foreground hover:bg-primary/90",
        secondary: "border border-border bg-muted text-foreground hover:bg-accent",
        ghost: "text-foreground-muted hover:bg-muted hover:text-foreground",
        danger:
          "border border-danger-border bg-danger-muted text-danger-foreground hover:bg-danger-border/40"
      },
      size: { default: "h-9 px-3.5", sm: "h-8 px-3 text-xs", icon: "size-9" }
    },
    defaultVariants: { variant: "default", size: "default" }
  }
);
```

`src/components/ui/input.tsx`, the class string:

```ts
"h-9 w-full rounded-md border border-border bg-card px-3 text-sm text-foreground outline-none placeholder:text-foreground-faint focus:border-border-strong focus:ring-1 focus:ring-ring/40",
```

`src/components/ui/status-badge.tsx`:

```ts
const statusStyles: Record<string, string> = {
  working: "bg-status-running",
  running: "bg-status-running",
  waiting: "bg-status-waiting",
  paused: "bg-status-neutral",
  pending: "bg-status-neutral",
  queued: "bg-status-queued",
  failed: "bg-status-danger",
  blocked: "bg-status-danger",
  completed: "bg-status-success",
  approved: "bg-status-success",
  denied: "bg-status-danger",
  cancelled: "bg-status-neutral/70",
  idle: "bg-status-neutral"
};
```

In the same file, change `text-zinc-400` to `text-foreground-muted` and the fallback `"bg-zinc-500"` to `"bg-status-neutral"`.

`src/features/agents/agent-avatar.tsx`:

```ts
const colors: Record<AgentColor, string> = {
  indigo:
    "from-identity-indigo/25 to-identity-indigo/5 text-identity-indigo ring-identity-indigo/25",
  cyan: "from-identity-cyan/25 to-identity-cyan/5 text-identity-cyan ring-identity-cyan/25",
  emerald:
    "from-identity-emerald/25 to-identity-emerald/5 text-identity-emerald ring-identity-emerald/25",
  amber: "from-identity-amber/25 to-identity-amber/5 text-identity-amber ring-identity-amber/25",
  rose: "from-identity-rose/25 to-identity-rose/5 text-identity-rose ring-identity-rose/25",
  violet:
    "from-identity-violet/25 to-identity-violet/5 text-identity-violet ring-identity-violet/25"
};

const statusColors: Record<AgentStatus, string> = {
  idle: "bg-status-neutral",
  working: "bg-status-running",
  waiting: "bg-status-waiting",
  paused: "bg-status-neutral",
  failed: "bg-status-danger",
  completed: "bg-status-success"
};
```

In the same file, change the status dot's `border-zinc-950` to `border-background`.

`src/features/agents/new-agent-dialog.tsx`, the `swatches` map and the selected-swatch class:

```ts
const swatches: Record<AgentColor, string> = {
  indigo: "bg-identity-indigo",
  cyan: "bg-identity-cyan",
  emerald: "bg-identity-emerald",
  amber: "bg-identity-amber",
  rose: "bg-identity-rose",
  violet: "bg-identity-violet"
};
```

Change `"border-white scale-110"` to `"border-foreground scale-110"`.

`src/app/sidebar.tsx`:

| Before                                                                                                    | After                                                                                                    |
| --------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------- |
| `border-zinc-900 bg-[#0b0b0d]` (aside)                                                                    | `border-border-subtle bg-surface`                                                                        |
| logo `border-zinc-700 bg-zinc-100`                                                                        | `border-border-strong bg-primary`                                                                        |
| logo icon `text-zinc-950`                                                                                 | `text-primary-foreground`                                                                                |
| `text-zinc-100` (product name)                                                                            | `text-foreground`                                                                                        |
| alpha badge `bg-zinc-900 … text-[9px] … text-zinc-600`                                                    | `bg-muted … text-3xs … text-foreground-faint`                                                            |
| active nav `bg-zinc-800/80 text-zinc-100`                                                                 | `bg-accent text-foreground`                                                                              |
| inactive nav `text-zinc-500 hover:bg-zinc-900 hover:text-zinc-300`                                        | `text-foreground-subtle hover:bg-muted hover:text-foreground-secondary`                                  |
| approvals dot `bg-amber-400`                                                                              | `bg-status-waiting`                                                                                      |
| commands button `border-zinc-900 bg-zinc-950 … text-zinc-600 … hover:border-zinc-800 hover:text-zinc-400` | `border-border-subtle bg-card … text-foreground-faint … hover:border-border hover:text-foreground-muted` |
| kbd `border-zinc-800 … text-[9px]`                                                                        | `border-border … text-3xs`                                                                               |
| footer `text-[10px] text-zinc-700`                                                                        | `text-2xs text-foreground-faint`                                                                         |
| runtime dot `bg-emerald-500`                                                                              | `bg-status-success`                                                                                      |

- [ ] **Step 4: Migrate the remaining files with this mapping**

Apply it to every remaining violation reported in Step 2. Variant prefixes (`hover:`, `focus:`, `placeholder:`, `before:`, `group-hover:`) are kept unchanged.

| Raw class                                               | Token class                              |
| ------------------------------------------------------- | ---------------------------------------- |
| `bg-zinc-950`, `bg-zinc-950/N`                          | `bg-card`, `bg-card/N`                   |
| `bg-zinc-900`, `bg-zinc-900/N`                          | `bg-muted`, `bg-muted/N`                 |
| `bg-zinc-800`, `bg-zinc-800/80`                         | `bg-accent`                              |
| `bg-zinc-800` used as a 1px line (`before:bg-zinc-800`) | `before:bg-border`                       |
| `bg-black/60`, `bg-black/65` (modal backdrop)           | `bg-overlay`                             |
| `bg-black/40` (code block)                              | `bg-muted`                               |
| `border-zinc-950`                                       | `border-background`                      |
| `border-zinc-900`                                       | `border-border-subtle`                   |
| `border-zinc-800`, `border-zinc-800/80`                 | `border-border`, `border-border/80`      |
| `border-zinc-700`, `border-zinc-600`                    | `border-border-strong`                   |
| `ring-zinc-700`                                         | `ring-ring/40`                           |
| `text-zinc-100`, `text-zinc-200`                        | `text-foreground`                        |
| `text-zinc-300`                                         | `text-foreground-secondary`              |
| `text-zinc-400`                                         | `text-foreground-muted`                  |
| `text-zinc-500`                                         | `text-foreground-subtle`                 |
| `text-zinc-600`, `text-zinc-700`                        | `text-foreground-faint`                  |
| `accent-zinc-300` (checkbox)                            | `accent-foreground`                      |
| `border-red-900/60`, `border-red-900/70`                | `border-danger-border`                   |
| `bg-red-950/20`, `bg-red-950/40`                        | `bg-danger-muted`                        |
| `text-red-300`                                          | `text-danger-foreground`                 |
| `text-red-400`                                          | `text-danger`                            |
| `border-amber-900/50`                                   | `border-warning-border`                  |
| `bg-amber-950/20`                                       | `bg-warning-muted`                       |
| `text-amber-300/80`                                     | `text-warning-foreground`                |
| `text-amber-500/70`                                     | `text-warning`                           |
| `text-[9px]` / `text-[10px]` / `text-[11px]`            | `text-3xs` / `text-2xs` / `text-xs-plus` |

Concrete occurrences, so nothing is missed:

- `src/app/app.tsx`: the loading text `text-zinc-600` becomes `text-foreground-faint`. The error box `border-red-900/60 bg-red-950/20 … text-red-300` becomes `border-danger-border bg-danger-muted … text-danger-foreground`.
- `src/app/command-palette.tsx`:
  - backdrop `bg-black/60` → `bg-overlay`;
  - panel `border-zinc-800 bg-zinc-950` → `border-border bg-card`;
  - header `border-zinc-900` → `border-border-subtle`;
  - icons `text-zinc-600` and `text-zinc-700` → `text-foreground-faint`;
  - input `text-zinc-200 … placeholder:text-zinc-600` → `text-foreground … placeholder:text-foreground-faint`;
  - section label `text-[10px] … text-zinc-700` → `text-2xs … text-foreground-faint`;
  - command `text-zinc-400 hover:bg-zinc-900 hover:text-zinc-100` → `text-foreground-muted hover:bg-muted hover:text-foreground`;
  - kbd `text-[10px] text-zinc-700` → `text-2xs text-foreground-faint`.
- `src/features/updates/update-banner.tsx`:
  - error tone → `border-danger-border bg-danger-muted … text-danger-foreground`;
  - default tone `border-zinc-800/80 bg-zinc-950 … text-zinc-300` → `border-border/80 bg-card … text-foreground-secondary`;
  - dismiss `text-zinc-500 hover:text-zinc-200` → `text-foreground-subtle hover:text-foreground`.
- `src/features/agents/agents-page.tsx`: the demo notice `border-amber-900/50 bg-amber-950/20 … text-amber-300/80` becomes `border-warning-border bg-warning-muted … text-warning-foreground`. All other classes in the file follow the table.
- `src/features/approvals/approvals-page.tsx`: the `pre` block `border-zinc-800 bg-black/40 … text-zinc-200` becomes `border-border bg-muted … text-foreground`, and `text-amber-500/70` becomes `text-warning`.
- `activity-page.tsx`, `tasks-page.tsx`, `settings-page.tsx`, `agent-details.tsx`, `new-agent-dialog.tsx`, `update-settings.tsx`: use the table only.

- [ ] **Step 5: Run the scan until it is clean**

Run: `npx vitest run src/styles/design-tokens.test.ts`
Expected: PASS. If violations remain, map them with the table and rerun.

- [ ] **Step 6: Visual check of both themes**

Run: `npm run dev` and open `http://localhost:1420`. The pre-paint script does not exist yet, so the page starts in light. In DevTools, run `document.documentElement.classList.toggle("dark")` to switch.

Check each view (Agents, Agent details, Tasks, Activity, Approvals, Settings, the New Agent dialog, and the ⌘K palette) in both themes:

- Dark must look like it did before migration.
- Light: text must be readable, borders visible, status dots distinguishable, and avatar identity colors must stay distinct from status colors.
- If a token value fails, adjust it **only in `src/styles/tokens.css`**.

Stop the dev server afterwards.

- [ ] **Step 7: Full frontend checks**

Run: `npm run format && npm run lint && npm run typecheck && npm test`
Expected: all pass.

- [ ] **Step 8: Checkpoint (no commit)**

Run: `git status --short`. Expected: Task 1-2 files plus the 16 component files listed above.

---

### Task 4: Pre-paint theme script

**Files:**

- Create: `public/theme-init.js`
- Modify: `index.html`
- Modify: `eslint.config.js`
- Test: `src/lib/theme-init.test.ts`

**Interfaces:**

- Consumes: `themeStorageKey`, `systemDarkQuery` from `src/lib/theme.ts` (in the test, to prove the two stay in sync).
- Produces: `<html>` has the correct `dark` class and `color-scheme` before React mounts.

- [ ] **Step 1: Write the failing test** — create `src/lib/theme-init.test.ts`:

```ts
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
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npx vitest run src/lib/theme-init.test.ts`
Expected: FAIL — `ENOENT ... public/theme-init.js`.

- [ ] **Step 3: Create `public/theme-init.js`**

```js
// Applies the stored theme before the first paint so the window never flashes the wrong theme.
// Keep the storage key and fallback in sync with src/lib/theme.ts (covered by theme-init.test.ts).
(function () {
  var preference = "dark";
  try {
    var stored = window.localStorage.getItem("open-bots.theme");
    if (stored === "system" || stored === "light" || stored === "dark") preference = stored;
  } catch {
    // Storage can be unavailable; keep the default theme.
  }
  var systemDark =
    typeof window.matchMedia === "function" &&
    window.matchMedia("(prefers-color-scheme: dark)").matches;
  var dark = preference === "dark" || (preference === "system" && systemDark);
  document.documentElement.classList.toggle("dark", dark);
  document.documentElement.style.colorScheme = dark ? "dark" : "light";
})();
```

- [ ] **Step 4: Update `index.html`**

Replace `<html lang="en" class="dark">` with `<html lang="en">`. Add `<script src="/theme-init.js"></script>` as the last element inside `<head>`, after `<title>`. A classic external script is required because the Tauri CSP (`default-src 'self'`) blocks inline scripts. Vite logs a notice that it cannot bundle a non-module script; that is expected, and the file is served from `public/` as-is.

- [ ] **Step 5: Give `public/**/*.js` browser globals in `eslint.config.js`**

Add this entry after the `files: ["**/*.{js,mjs,cjs}"]` block:

```js
  {
    files: ["public/**/*.js"],
    languageOptions: { globals: globals.browser }
  },
```

- [ ] **Step 6: Run tests, lint, and build**

Run: `npx vitest run src/lib/theme-init.test.ts && npm run lint && npm run build && ls dist/theme-init.js`
Expected: tests PASS, lint clean, build succeeds, `dist/theme-init.js` exists.

- [ ] **Step 7: Checkpoint (no commit)**

Run: `git status --short`. Expected new entries: `public/theme-init.js`, `src/lib/theme-init.test.ts`, `index.html`, `eslint.config.js`.

---

### Task 5: Motion dependency, presets, and provider

**Files:**

- Modify: `package.json`, `package-lock.json` (via npm)
- Create: `src/lib/motion.ts`
- Test: `src/lib/motion.test.ts`
- Create: `src/app/motion-provider.tsx`
- Modify: `src/main.tsx`
- Modify: `docs/dependency-audit.md`

**Interfaces:**

- Consumes: `durations`, `maxDuration`, `easings` from `src/styles/motion-tokens.ts`.
- Produces:
  - `transitions: { enter: Transition; exit: Transition }`
  - `fadeUp: Variants`, `scaleIn: Variants`, `iconSwap: Variants`, each with keys `initial`, `animate`, `exit`
  - `MotionProvider({ children }: { children: ReactNode })`

- [ ] **Step 1: Install the dependency**

Run: `npm install motion@^13.4.6`
Expected: `"motion": "^13.4.6"` in `dependencies`.

- [ ] **Step 2: Write the failing test** — create `src/lib/motion.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import { fadeUp, iconSwap, scaleIn } from "./motion";
import { maxDuration } from "@/styles/motion-tokens";

const presets = { fadeUp, scaleIn, iconSwap };
const animatable = new Set(["opacity", "x", "y", "scale", "rotate"]);

describe("motion presets", () => {
  it.each(Object.entries(presets))("%s stays within the duration ceiling", (_name, variants) => {
    for (const target of Object.values(variants) as unknown as Record<string, unknown>[]) {
      const transition = target.transition as { duration?: number } | undefined;
      expect((transition?.duration ?? 0) * 1000).toBeLessThanOrEqual(maxDuration);
    }
  });

  it.each(Object.entries(presets))("%s animates only opacity and transforms", (_name, variants) => {
    for (const target of Object.values(variants) as unknown as Record<string, unknown>[]) {
      const properties = Object.keys(target).filter((key) => key !== "transition");
      expect(properties.filter((key) => !animatable.has(key))).toEqual([]);
    }
  });
});
```

- [ ] **Step 3: Run test to verify it fails**

Run: `npx vitest run src/lib/motion.test.ts`
Expected: FAIL — `Failed to resolve import "./motion"`.

- [ ] **Step 4: Create `src/lib/motion.ts`**

```ts
import type { Transition, Variants } from "motion/react";
import { durations, easings } from "@/styles/motion-tokens";

// Motion conventions are documented in docs/design-system.md#motion.
const seconds = (milliseconds: number) => milliseconds / 1000;

export const transitions = {
  enter: { duration: seconds(durations.base), ease: easings.standard },
  exit: { duration: seconds(durations.fast), ease: easings.exit }
} satisfies Record<string, Transition>;

/** Content appearing in place: pages, panels, banners. */
export const fadeUp: Variants = {
  initial: { opacity: 0, y: 4 },
  animate: { opacity: 1, y: 0, transition: transitions.enter },
  exit: { opacity: 0, y: 4, transition: transitions.exit }
};

/** Overlays anchored to the viewport: dialogs and the command palette. */
export const scaleIn: Variants = {
  initial: { opacity: 0, scale: 0.98 },
  animate: { opacity: 1, scale: 1, transition: transitions.enter },
  exit: { opacity: 0, scale: 0.98, transition: transitions.exit }
};

/** Two icons replacing each other in the same slot, such as the theme toggle. */
export const iconSwap: Variants = {
  initial: { opacity: 0, rotate: -90, scale: 0.6 },
  animate: { opacity: 1, rotate: 0, scale: 1, transition: transitions.enter },
  exit: { opacity: 0, rotate: 90, scale: 0.6, transition: transitions.exit }
};
```

- [ ] **Step 5: Run test to verify it passes**

Run: `npx vitest run src/lib/motion.test.ts && npm run typecheck`
Expected: PASS; typecheck clean.

- [ ] **Step 6: Create `src/app/motion-provider.tsx`**

```tsx
import { LazyMotion, MotionConfig, domAnimation } from "motion/react";
import type { ReactNode } from "react";

/**
 * Loads only DOM animation features and honors the operating system's reduced-motion setting.
 * `strict` makes using `motion.*` instead of the lightweight `m.*` components an error.
 */
export function MotionProvider({ children }: { children: ReactNode }) {
  return (
    <LazyMotion features={domAnimation} strict>
      <MotionConfig reducedMotion="user">{children}</MotionConfig>
    </LazyMotion>
  );
}
```

- [ ] **Step 7: Wrap the app in `src/main.tsx`**

```tsx
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "@/app/app";
import { MotionProvider } from "@/app/motion-provider";
import "./index.css";

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <MotionProvider>
      <App />
    </MotionProvider>
  </StrictMode>
);
```

- [ ] **Step 8: Audit and record**

Run: `npm audit --audit-level=high`
Expected: exit 0. If it reports high or critical findings, stop and report them to the user instead of continuing.

In `docs/dependency-audit.md`, keep `Last checked: 2026-09-30` and add this paragraph under `## JavaScript`:

```markdown
`motion` (13.x) was added for UI animation. It has no runtime dependencies beyond its own `motion-dom` and `motion-utils` packages and is loaded through `LazyMotion` with DOM animation features only.
```

Before saving the sentence, verify it with `npm ls motion-dom motion-utils --all`. If the tree differs, describe what it actually shows.

- [ ] **Step 9: Build and checkpoint (no commit)**

Run: `npm run build && git status --short`
Expected: build succeeds. New/changed: `package.json`, `package-lock.json`, `src/lib/motion.ts`, `src/lib/motion.test.ts`, `src/app/motion-provider.tsx`, `src/main.tsx`, `docs/dependency-audit.md`.

---

### Task 6: ThemeProvider, useTheme, and native window sync

**Files:**

- Create: `src/hooks/use-theme.ts`
- Create: `src/app/theme-provider.tsx`
- Test: `src/app/theme-provider.test.tsx`
- Modify: `src/lib/desktop-api.ts`
- Modify: `src-tauri/capabilities/default.json`
- Modify: `src/main.tsx`

**Interfaces:**

- Consumes:
  - From Task 1: `applyTheme`, `oppositeTheme`, `readThemePreference`, `resolveTheme`, `systemDarkQuery`, `writeThemePreference`, `ThemePreference`, `ResolvedTheme`, `browserStorage`.
  - From the codebase: `isTauriRuntime()` in `desktop-api.ts`.
- Produces:
  - `interface ThemeContextValue { preference: ThemePreference; resolved: ResolvedTheme; setPreference: (preference: ThemePreference) => void; toggle: () => void }`
  - `ThemeContext`, `useTheme(): ThemeContextValue`
  - `ThemeProvider({ children }: { children: ReactNode })`
  - `setNativeWindowTheme(theme: ResolvedTheme | null): Promise<void>`

- [ ] **Step 1: Write the failing test** — create `src/app/theme-provider.test.tsx`:

```tsx
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
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npx vitest run src/app/theme-provider.test.tsx`
Expected: FAIL — `Failed to resolve import "./theme-provider"`.

- [ ] **Step 3: Create `src/hooks/use-theme.ts`**

```ts
import { createContext, useContext } from "react";
import type { ResolvedTheme, ThemePreference } from "@/lib/theme";

export interface ThemeContextValue {
  preference: ThemePreference;
  resolved: ResolvedTheme;
  setPreference: (preference: ThemePreference) => void;
  toggle: () => void;
}

export const ThemeContext = createContext<ThemeContextValue | undefined>(undefined);

export function useTheme(): ThemeContextValue {
  const value = useContext(ThemeContext);
  if (!value) throw new Error("useTheme must be used inside ThemeProvider.");
  return value;
}
```

- [ ] **Step 4: Add `setNativeWindowTheme` to `src/lib/desktop-api.ts`**

Add these imports at the top, keeping the existing ones:

```ts
import { getCurrentWindow } from "@tauri-apps/api/window";
import type { ResolvedTheme } from "@/lib/theme";
```

Append this function:

```ts
/** Aligns the native title bar with the app theme. `null` lets it follow the operating system. */
export async function setNativeWindowTheme(theme: ResolvedTheme | null): Promise<void> {
  if (!isTauriRuntime()) return;
  await getCurrentWindow().setTheme(theme);
}
```

- [ ] **Step 5: Create `src/app/theme-provider.tsx`**

```tsx
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
```

- [ ] **Step 6: Run tests to verify they pass**

Run: `npx vitest run src/app/theme-provider.test.tsx`
Expected: PASS (8 tests).

- [ ] **Step 7: Grant the Tauri permission**

In `src-tauri/capabilities/default.json`:

```json
  "permissions": [
    "core:default",
    "core:window:allow-set-theme",
    "updater:default",
    "process:allow-restart"
  ]
```

- [ ] **Step 8: Wrap the app with `ThemeProvider` in `src/main.tsx`**

```tsx
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "@/app/app";
import { MotionProvider } from "@/app/motion-provider";
import { ThemeProvider } from "@/app/theme-provider";
import "./index.css";

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <ThemeProvider>
      <MotionProvider>
        <App />
      </MotionProvider>
    </ThemeProvider>
  </StrictMode>
);
```

- [ ] **Step 9: Frontend and Rust checks**

Run: `npm run lint && npm run typecheck && npm test`
Then: `cd src-tauri && cargo check --all-targets --all-features`
Expected: all clean. `cargo check` runs `tauri-build`, which validates the new capability identifier.

- [ ] **Step 10: Checkpoint (no commit)**

Run: `git status --short`. New/changed: `src/hooks/use-theme.ts`, `src/app/theme-provider.tsx`, `src/app/theme-provider.test.tsx`, `src/lib/desktop-api.ts`, `src-tauri/capabilities/default.json`, `src/main.tsx`.

---

### Task 7: Theme toggle, settings selector, and palette command

**Files:**

- Create: `src/features/appearance/theme-toggle.tsx`
- Create: `src/features/appearance/theme-selector.tsx`
- Test: `src/features/appearance/theme-toggle.test.tsx`
- Test: `src/features/appearance/theme-selector.test.tsx`
- Modify: `src/app/sidebar.tsx` (footer)
- Modify: `src/features/settings/settings-page.tsx` (Appearance card and `Row`)
- Modify: `src/app/command-palette.tsx` (new command, optional shortcut)
- Modify: `src/app/app.tsx` (pass `onToggleTheme`)

**Interfaces:**

- Consumes: `useTheme()` and `ThemeProvider` (Task 6); `MotionProvider` and `iconSwap` (Task 5); `themeStorageKey` and `ThemePreference` (Task 1).
- Produces:
  - `ThemeToggle()`: a button named `Switch to light theme` / `Switch to dark theme`.
  - `ThemeSelector()`: a radio group named `Theme` with radios `System`, `Light`, `Dark`.
  - `CommandPalette` gains the prop `onToggleTheme: () => void`.

- [ ] **Step 1: Write the failing tests**

`src/features/appearance/theme-toggle.test.tsx`:

```tsx
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
```

`src/features/appearance/theme-selector.test.tsx`:

```tsx
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
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `npx vitest run src/features/appearance`
Expected: FAIL — `Failed to resolve import "./theme-toggle"` and `"./theme-selector"`.

- [ ] **Step 3: Create `src/features/appearance/theme-toggle.tsx`**

```tsx
import { AnimatePresence, m } from "motion/react";
import { Moon, Sun } from "lucide-react";
import { useTheme } from "@/hooks/use-theme";
import { iconSwap } from "@/lib/motion";
import { oppositeTheme } from "@/lib/theme";

export function ThemeToggle() {
  const { resolved, toggle } = useTheme();
  const label = `Switch to ${oppositeTheme(resolved)} theme`;
  const Icon = resolved === "dark" ? Moon : Sun;
  return (
    <button
      type="button"
      onClick={toggle}
      aria-label={label}
      title={label}
      className="grid size-6 place-items-center overflow-hidden rounded-md text-foreground-subtle transition-colors hover:bg-muted hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
    >
      <AnimatePresence initial={false} mode="popLayout">
        <m.span
          key={resolved}
          variants={iconSwap}
          initial="initial"
          animate="animate"
          exit="exit"
          className="grid place-items-center"
        >
          <Icon size={13} strokeWidth={1.8} aria-hidden="true" />
        </m.span>
      </AnimatePresence>
    </button>
  );
}
```

- [ ] **Step 4: Create `src/features/appearance/theme-selector.tsx`**

```tsx
import { Monitor, Moon, Sun } from "lucide-react";
import { useRef, type KeyboardEvent } from "react";
import { useTheme } from "@/hooks/use-theme";
import type { ThemePreference } from "@/lib/theme";
import { cn } from "@/lib/utils";

const options = [
  { value: "system", label: "System", icon: Monitor },
  { value: "light", label: "Light", icon: Sun },
  { value: "dark", label: "Dark", icon: Moon }
] satisfies { value: ThemePreference; label: string; icon: typeof Sun }[];

const keyOffsets: Record<string, number> = {
  ArrowRight: 1,
  ArrowDown: 1,
  ArrowLeft: -1,
  ArrowUp: -1
};

export function ThemeSelector() {
  const { preference, setPreference } = useTheme();
  const buttons = useRef<(HTMLButtonElement | null)[]>([]);

  function handleKeyDown(event: KeyboardEvent<HTMLButtonElement>, index: number) {
    const offset = keyOffsets[event.key];
    if (!offset) return;
    event.preventDefault();
    const next = (index + offset + options.length) % options.length;
    setPreference(options[next].value);
    buttons.current[next]?.focus();
  }

  return (
    <div
      role="radiogroup"
      aria-label="Theme"
      className="inline-flex rounded-md border border-border bg-muted p-0.5"
    >
      {options.map(({ value, label, icon: Icon }, index) => {
        const checked = preference === value;
        return (
          <button
            key={value}
            ref={(element) => {
              buttons.current[index] = element;
            }}
            type="button"
            role="radio"
            aria-checked={checked}
            tabIndex={checked ? 0 : -1}
            onClick={() => setPreference(value)}
            onKeyDown={(event) => handleKeyDown(event, index)}
            className={cn(
              "flex h-6 items-center gap-1.5 rounded px-2 text-xs transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
              checked ? "bg-card text-foreground" : "text-foreground-subtle hover:text-foreground"
            )}
          >
            <Icon size={12} aria-hidden="true" />
            {label}
          </button>
        );
      })}
    </div>
  );
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `npx vitest run src/features/appearance`
Expected: PASS (5 tests).

- [ ] **Step 6: Wire the UI**

In `src/app/sidebar.tsx`, import `ThemeToggle` from `@/features/appearance/theme-toggle` and replace the footer status row (the last `<div className="mt-3 …">`, already migrated in Task 3) with:

```tsx
<div className="mt-3 flex items-center justify-between pl-2">
  <span className="flex items-center gap-2 text-2xs text-foreground-faint">
    <span className="size-1.5 rounded-full bg-status-success" /> Local runtime
  </span>
  <ThemeToggle />
</div>
```

In `src/features/settings/settings-page.tsx`:

- import `ThemeSelector` from `@/features/appearance/theme-selector`;
- replace `<Row name="Theme" value="Dark" />` with `<Row name="Theme" value={<ThemeSelector />} />`;
- change `Row` so `value` is a `React.ReactNode` and is rendered in a `<div className="text-xs text-foreground-secondary">` instead of a `<span>` (a radio group must not sit inside a `span`).

In `src/app/command-palette.tsx`:

- add `onToggleTheme: () => void` to the props, after `onCreateAgent`;
- import `SunMoon` from `lucide-react`;
- add this command after "View Tasks":

```tsx
<Command icon={SunMoon} label="Toggle Theme" onClick={() => execute(onToggleTheme)} />
```

- in `Command`, make the prop `shortcut?: string` and render the `<kbd>` only when `shortcut` is set: `{shortcut && <kbd className="ml-auto text-2xs text-foreground-faint">{shortcut}</kbd>}`. This avoids displaying a shortcut that does not exist.

In `src/app/app.tsx`:

- import `useTheme` from `@/hooks/use-theme`;
- add `const { toggle: toggleTheme } = useTheme();` next to `const updater = useAppUpdater();`;
- pass `onToggleTheme={toggleTheme}` to `<CommandPalette>`.

- [ ] **Step 7: Full frontend checks**

Run: `npm run format && npm run format:check && npm run lint && npm run typecheck && npm test && npm run build`
Expected: all pass.

- [ ] **Step 8: Manual check in the browser**

Run `npm run dev`, open `http://localhost:1420`, and verify:

1. First load is dark, with no flash.
2. The sidebar toggle animates sun/moon (rotate and fade, under 0.25s), and every surface changes in the same frame without a color cascade.
3. Reloading keeps the choice.
4. _Settings → Appearance_: System / Light / Dark works with mouse and with arrow keys. Under System, switching the OS (or the DevTools "Emulate CSS prefers-color-scheme" option) updates the app live.
5. ⌘K → "Toggle Theme" works.
6. With DevTools "Emulate CSS prefers-reduced-motion: reduce", the icon only crossfades.

Stop the dev server.

- [ ] **Step 9: Checkpoint (no commit)**

Run: `git status --short`. New/changed: `src/features/appearance/*` (4 files), `sidebar.tsx`, `settings-page.tsx`, `command-palette.tsx`, `app.tsx`.

---

### Task 8: Documentation and full quality gates

**Files:**

- Create: `docs/adr/0005-design-tokens-and-motion.md`
- Create: `docs/design-system.md`
- Modify: `docs/architecture.md` (append section)

**Interfaces:**

- Consumes: everything above.
- Produces: documentation only.

- [ ] **Step 1: Create `docs/adr/0005-design-tokens-and-motion.md`**

```markdown
# ADR 0005: Design tokens as a single source, and Motion for UI animation

- Status: Accepted
- Date: 2026-09-30

## Context

The desktop UI hard-coded Tailwind palette classes (`zinc-*`, `red-*`) in every component and supported only a dark theme. Adding a light theme, or changing the visual identity later, would have required editing every component. The UI also needs a consistent way to animate interactions without drifting from the calm, compact style defined in `AGENTS.md`.

## Decision

All colors, font families, small font sizes, radii, and shadows are CSS custom properties in `src/styles/tokens.css`, with `:root` as the light theme and `.dark` as the dark theme. Tailwind exposes them as semantic utilities (`bg-card`, `text-foreground-muted`, `bg-status-running`, `text-identity-indigo`). Components must not use raw palette values; a test enforces this. Runtime status colors and agent identity colors are separate token groups.

The theme preference (`system`, `light`, or `dark`, default `dark`) is a per-device UI preference stored in `localStorage`, not in SQLite. It carries no domain meaning and must be readable before React loads, which `public/theme-init.js` does to avoid a flash of the wrong theme.

UI animation uses the `motion` package (`motion/react`), the successor of Framer Motion. It is loaded through `LazyMotion` with DOM animation features in strict mode, and `MotionConfig` honors the operating system's reduced-motion setting. Durations and easings live in `src/styles/motion-tokens.ts` and are shared with Tailwind.

## Consequences

Changing the design system means editing one file. New components inherit both themes automatically. The theme preference is not shared across devices or backed up with application data. `motion` adds a runtime dependency that must be included in dependency audits. The native title bar follows the theme through `core:window:allow-set-theme`. On Linux and macOS, Tauri applies the theme app-wide, and Linux behavior depends on the desktop environment.
```

- [ ] **Step 2: Create `docs/design-system.md`**

```markdown
# Design System

The visual identity of Open Bots is defined in one file: `src/styles/tokens.css`. Components use semantic Tailwind utilities mapped in `tailwind.config.ts` and never raw palette colors. `src/styles/design-tokens.test.ts` enforces this.

## Changing the look

| To change            | Edit in `src/styles/tokens.css`                                                                          |
| -------------------- | -------------------------------------------------------------------------------------------------------- |
| A color in one theme | The variable inside `:root` (light) or `.dark` (dark). Values are HSL channels, for example `240 6% 4%`. |
| Font families        | `--font-sans`, `--font-mono`                                                                             |
| Small text sizes     | `--text-3xs`, `--text-2xs`, `--text-xs-plus`                                                             |
| Corner radius        | `--radius-sm`, `--radius-md`, `--radius-lg`, `--radius-xl`                                               |
| Panel shadow         | `--shadow-panel`                                                                                         |

Adding a new token requires declaring it in `:root` and, for colors, in `.dark`, then mapping it in `tailwind.config.ts`. The token tests fail if a theme is missing a value.

## Token groups

- **Surfaces:** `background`, `surface`, `card`, `muted`, `accent`, `overlay`.
- **Borders:** `border`, `border-subtle`, `border-strong`, `ring`, `selection`.
- **Text:** `foreground`, `foreground-secondary`, `foreground-muted`, `foreground-subtle`, `foreground-faint`.
- **Primary action:** `primary`, `primary-foreground`.
- **Runtime status:** `status-running`, `status-waiting`, `status-queued`, `status-success`, `status-danger`, `status-neutral`. Only for runtime state.
- **Feedback surfaces:** `danger-*`, `warning-*` (`DEFAULT`, `foreground`, `muted`, `border`).
- **Agent identity:** `identity-indigo`, `-cyan`, `-emerald`, `-amber`, `-rose`, `-violet`. Never used to show status.

## Themes

Users choose System, Light, or Dark in Settings → Appearance, or toggle from the sidebar or the command palette. Dark is the default. The choice is stored per device in `localStorage` under `open-bots.theme`.

## Motion

Use `motion/react` with `m.*` components (the app runs inside `LazyMotion strict`). Timing comes from `src/styles/motion-tokens.ts`. Presets are in `src/lib/motion.ts`:

- `fadeUp` for content appearing in place;
- `scaleIn` for dialogs and the command palette;
- `iconSwap` for icons replacing each other.

Rules:

- Animate only `opacity` and transforms.
- Keep every animation at or under 240ms (`durations.slow`).
- Animate to show the result of a user action, not for decoration.
- Do not animate entrances on re-render or across large lists.
- Never use motion to suggest that an unsupported runtime capability is active.
- Reduced motion is handled globally by `MotionConfig reducedMotion="user"`; do not override it.
- Use the Tailwind `duration-fast/base/slow` and `ease-standard/exit` utilities for CSS transitions.
```

- [ ] **Step 3: Append to `docs/architecture.md`**

```markdown
## Frontend design system

Visual values come from `src/styles/tokens.css` and are exposed as semantic Tailwind utilities. Theme state lives in `ThemeProvider` (`src/app/theme-provider.tsx`) with pure logic in `src/lib/theme.ts`. See [design-system.md](design-system.md) and [ADR 0005](adr/0005-design-tokens-and-motion.md).
```

- [ ] **Step 4: Run every required quality gate**

Frontend:

```bash
npm run format:check
npm run lint
npm run typecheck
npm test
npm run build
npm audit --audit-level=high
```

Rust (the capability file changed):

```bash
cd src-tauri
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
cargo check --all-targets --all-features
```

Expected: all pass. Record any gate that could not run on this WSL host, and why, for the final report. Never report an unexecuted gate as passed.

- [ ] **Step 5: Desktop check (if the host supports it)**

Run: `npm run tauri dev`. Verify that the title bar follows Light / Dark and that System follows the OS. If WSLg or GTK prerequisites are missing, report that the native title-bar sync was not verified on this host.

- [ ] **Step 6: Final diff review (no commit)**

Run: `git status --short && git diff --stat`
Review against the AGENTS.md architecture checklist:

- no SQL or shell outside infrastructure;
- no business logic in components;
- no secrets in logs (the `console.warn` logs only the Tauri error);
- no unrelated changes;
- documentation matches the behavior.

Do not commit. Report:

- the changed files;
- which gates ran and their results;
- the remaining limitations: Linux title-bar behavior, and the `animate-fade-in` class not yet migrated to Motion.
