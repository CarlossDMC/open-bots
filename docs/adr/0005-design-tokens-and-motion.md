# ADR 0005: Design tokens as a single source, and Motion for UI animation

- Status: Accepted
- Date: 2026-09-30

## Context

The desktop UI hard-coded Tailwind palette classes (`zinc-*`, `red-*`) in every component and supported only a dark theme. Adding a light theme, or changing the visual identity later, would have required editing every component. The UI also needs a consistent way to animate interactions without drifting from the calm, compact style defined in `AGENTS.md`.

## Decision

All colors, font families, small font sizes, radii, and shadows are CSS custom properties in `src/styles/tokens.css`, with `:root` as the light theme and `.dark` as the dark theme. Tailwind exposes them as semantic utilities (`bg-card`, `text-foreground-muted`, `bg-status-running`, `text-identity-indigo`). Components must not use raw palette values; a test enforces this. Runtime status colors and agent identity colors are separate token groups.

The theme preference (`system`, `light`, or `dark`, default `dark`) is a per-device UI preference stored in `localStorage`, not in SQLite. It carries no domain meaning and must be readable before React loads, which `public/theme-init.js` does to avoid a flash of the wrong theme.

UI animation uses the `motion` package through its `motion/react` entry point. `motion` is the current name of Framer Motion and wraps the `framer-motion` package of the same version. It is loaded through `LazyMotion` with DOM animation features in strict mode, and `MotionConfig` honors the operating system's reduced-motion setting. Durations and easings live in `src/styles/motion-tokens.ts` and are shared with Tailwind.

## Consequences

Changing the design system means editing one file. New components inherit both themes automatically. The theme preference is not shared across devices or backed up with application data. `motion` adds a runtime dependency (about 25 kB gzip in the current bundle) that must be included in dependency audits. The native title bar follows the theme through `core:window:allow-set-theme`. On Linux and macOS, Tauri applies the theme app-wide, and Linux behavior depends on the desktop environment. Agent avatars are the one place with ambient, repeating motion (blinking and a status-driven gaze); it stays within the per-step duration ceiling and is disabled by reduced motion.
