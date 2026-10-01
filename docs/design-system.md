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

## Layout

The app shell is a conversation layout:

- **Sidebar** (`src/app/sidebar.tsx`): a "New agent" button, a search field that filters agents by name or role, and the agent list. Each row shows the avatar, name, last update time, and the current task (or role). The footer holds the local runtime indicator and icon buttons for Tasks, Activity, Approvals, Settings, and the theme toggle. Every view is also reachable from the command palette (⌘K).
- **Conversation pane** (`src/features/chat/agent-conversation.tsx`): a slim header with the agent avatar, name, and status, plus a toggle for the agent details. The centered timeline shows only data the runtime already has (creation, current task, status). The pill composer is disabled and labeled "Messaging is not implemented yet" until a messaging backend exists.

## Agent avatars

Avatars are rounded squares filled with the agent identity color and a two-eyed face (`--avatar-eye`). `avatarVariant` picks the eye style (`orbital` dots, `signal` pills, `halo` wide ovals; other variants map deterministically). The eyes follow runtime status (`src/features/agents/avatar-expression.ts`): idle and completed blink, working scans side to side, waiting looks up, paused closes its eyes, and failed shows crossed eyes. The status dot remains the primary status signal.

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
- Reduced motion is honored for Motion components by `MotionConfig reducedMotion="user"`; do not override it. CSS animations and transitions (including the legacy `animate-fade-in`) do not respect it yet, so prefer `motion-safe:` for any new CSS animation.
- Exception: agent avatars (`src/features/agents/agent-avatar.tsx`) run ambient micro-animations through `avatarMotion` in `src/lib/motion.ts`. Blinking and the status gaze repeat after pauses, each step stays within 240ms, the gaze reflects only real runtime status, and reduced motion turns them off.
- Use the Tailwind `duration-fast/base/slow` and `ease-standard/exit` utilities for CSS transitions.
