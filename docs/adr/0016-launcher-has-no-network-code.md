# 0016. Launcher has no network code

## Status

Accepted (2026-11-11). Inherited from the family catalog (app-blueprint: `network-policy`, option no network); reference implementation: TimeTracker ADR 0013.

## Context

Launcher reads project folders, git metadata and local tool state (Docker, DDEV, wp-env) and starts local apps,
terminals and scripts. Nothing it does needs a remote host. The prototype made no HTTP calls either; its only
listener was a presentation hook that is not part of this app.

## Decision

Launcher contains no HTTP client and no listener beyond its own Unix socket (ADR 0009). The webview cannot
reach the network (CSP `connect-src ipc: http://ipc.localhost`, ADR 0017). Links such as a project's website
or repository open in the browser through `tauri-plugin-opener` (`open_url`, https only). No telemetry, update
check or crash reporting. Unit tests in `lib.rs` scan `Cargo.toml` and `cargo tree` and fail on reqwest,
hyper, ureq or an updater plugin. `docs/adapting.md` → Network hosts says "None". Git commands run locally;
Launcher never fetches or pushes.

- Rejected: listed hosts (no host is needed; repository info from a hosting API is not worth a token and a
  gateway); user-configured hosts only (there is nothing the owner would configure).

## Consequences

- No secrets, no gateway, no scopes page to maintain.
- A feature that needs the network later (e.g. repository status from a hosting service) needs a new ADR
  that supersedes this one, an HTTP client module and a row in Network hosts.
- Scripts the owner registers may use the network; they are the owner's processes, not Launcher's code.
