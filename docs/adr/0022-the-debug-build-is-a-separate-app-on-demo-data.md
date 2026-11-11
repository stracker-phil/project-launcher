# 0022. The debug build is a separate app on demo data

## Status

Accepted (2026-11-11). Inherited from the family catalog (app-blueprint: `debug-build-on-demo-data`); reference implementation: Taskboard ADR 0011 (identity consts) and
TimeTracker ADR 0031 (first-run seed, `_REAL_` switch).

## Context

The repository is public, and screenshots, e2e runs and agents drive the debug build. Launcher's real data is a
list of the owner's project folders, their names and their scripts; none of that may appear in a debug run.
Launching apps, running scripts and starting Docker from a debug build would also touch the owner's machine.

## Decision

The debug build is its own app: "Launcher (Debug)", `com.philippstracker.launcher.debug`, scheme
`launcher-debug`, its own config dir, socket, lock, logs and privacy grants; it runs beside the release app and
is never installed. `lib.rs` `IDENTIFIER`, `APP_NAME`, `LINK_SCHEME` are cfg consts tested against both Tauri
configs. On first launch with an empty data folder it creates made-up project folders inside its own data dir
(`test_support/demo/`: a few fictional projects with git history, `package.json`/`composer.json`, a
`.project-launcher.json`, tags and notes) and registers them; `launcher dev seed --yes` starts over.

Every outside source is faked by default: git stats come from the demo repositories, Docker/DDEV/wp-env from a
fake environment, the editor, terminal, git client and Finder launchers record the request and show it in a
toast, and DataBee is a stand-in script (the debug build never calls the owner's `databee`). Each has an
explicit switch, `LAUNCHER_REAL_<SOURCE>=1` (`_REAL_LAUNCH`, `_REAL_DOCKER`, `_REAL_DATABEE`), for checking
real-source code. The debug build never imports the release app's registry.

- Rejected: one shared identity; a shared data folder plus a flag; seeding only by hand; registering real
  folders in debug.

## Consequences

- Screenshots and agent sessions only ever show fictional projects.
- Every new outside source needs a fake and a switch in the same step.
