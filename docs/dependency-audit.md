# Dependency Audit

Last checked: 2026-10-01

## JavaScript

`npm audit --audit-level=high` reports no known vulnerabilities for the locked dependency tree.

`motion` (13.x) was added for UI animation. It wraps `framer-motion` of the same version, which depends on `motion-dom`, `motion-utils`, and `tslib`. The app loads it through `LazyMotion` with DOM animation features only.

`@fontsource-variable/inter` (5.x, OFL-1.1) bundles the Inter variable font locally, so the UI does not fetch fonts from the network. It has no runtime dependencies. On Apple platforms the system font (SF Pro) is used first.

## Rust

`cargo audit` reports no vulnerabilities and two allowed warnings in transitive dependencies:

- `glib 0.18.5` is affected by RUSTSEC-2024-0429. It enters through Tauri's Linux WebKit/GTK stack. Open Bots does not use the affected iterator API directly. Track the upstream Tauri GTK dependency and upgrade when a compatible release is available.
- `proc-macro-error 1.0.4` is marked unmaintained by RUSTSEC-2024-0370. It is a build-time transitive dependency. Track upstream replacement.

These warnings do not make the audit command fail, but they should be reviewed when Tauri dependencies are updated.
