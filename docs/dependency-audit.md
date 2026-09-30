# Dependency Audit

Last checked: 2026-09-30

## JavaScript

`npm audit --audit-level=high` reports no known vulnerabilities for the locked dependency tree.

## Rust

`cargo audit` reports no vulnerabilities and two allowed warnings in transitive dependencies:

- `glib 0.18.5` is affected by RUSTSEC-2024-0429. It enters through Tauri's Linux WebKit/GTK stack. Open Bots does not use the affected iterator API directly. Track the upstream Tauri GTK dependency and upgrade when a compatible release is available.
- `proc-macro-error 1.0.4` is marked unmaintained by RUSTSEC-2024-0370. It is a build-time transitive dependency. Track upstream replacement.

These warnings do not make the audit command fail, but they should be reviewed when Tauri dependencies are updated.
