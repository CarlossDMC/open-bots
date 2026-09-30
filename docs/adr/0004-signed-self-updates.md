# ADR 0004: Deliver signed self-updates from GitHub releases

- Status: Accepted
- Date: 2026-09-30

## Context

Desktop builds are published as GitHub releases. Without an update mechanism, users must notice and reinstall new versions by hand. Any update mechanism must keep core operation local-first and must not let a compromised download channel replace the application.

## Decision

Use the official Tauri updater and process plugins. The application reads a static `latest.json` manifest from the latest published GitHub release, and the release workflow generates it with `tauri-action`. Update artifacts are signed with a dedicated updater key: the public key is committed in `tauri.conf.json`, while the private key and its password live only in GitHub Actions secrets and the maintainer's local backup.

The check runs on launch unless the user disables it, and on demand from Settings. Installing always requires explicit user action and ends with a restart.

## Consequences

Update checks are the application's only network request that no provider or tool starts. Offline or failed checks are reported without affecting local operation.

The updater key becomes a critical release credential: losing it strands existing installations, and leaking it lets an attacker sign updates. Linux `.deb` and `.rpm` packages cannot self-update. The update logic lives in the frontend bridge (`src/lib/updater.ts`) because it concerns application distribution, not agent runtime or domain state.
