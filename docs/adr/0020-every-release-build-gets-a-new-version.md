# 0020. Every release build gets a new version

## Status

Accepted (2026-11-11). Superseded in part by [0039](0039-a-major-version-needs-the-owners-approval.md) (the bump rule). Inherited from the family catalog (app-blueprint: `release-versions`); reference implementation: Taskboard ADR 0054.

## Context

Launcher is installed to `~/Applications` and its CLI to `~/.local/bin`, and Daybook calls that CLI. The
installed version has to name one commit, and a change to the CLI's output must be visible as a version bump.

## Decision

`bun run release major|minor|patch|X.Y.Z` (`scripts/release.sh`) is the only way to release. It needs a clean
tree, moves the version in `package.json`, `tauri.conf.json`, `Cargo.toml` and `Cargo.lock`, builds, installs
(`scripts/install.sh`), commits "Release X.Y.Z", tags `vX.Y.Z` locally (never pushed) and runs
`scripts/clean-build.sh`. A failed build restores the version files; a tagged version is never built again.
Major when existing use breaks (the CLI and its `--json` shape, links, stored data without migration, a removed
feature), minor for new features or visible behaviour, patch otherwise. Semver from 1.0.0. A Rust test keeps
the three version strings equal. `install.sh` links the app and `~/.local/bin/launcher` (an unrelated
`launcher` command stays unless `--replace-cli`) and restarts a running release app, never a debug one.

- Rejected: a fixed version; build numbers or dates; bumps from commit prefixes; a bump per commit.

## Consequences

- Each install is distinguishable; Settings → About shows which commit runs.
- Releases are deliberate; work in progress runs only as the debug build.
