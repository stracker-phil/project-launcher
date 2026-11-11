# 0005. Panics unwind, so a bug fails one command and not the app

## Status

Accepted (2026-11-11). Inherited from the family catalog (app-blueprint: `panics-unwind`); reference
implementation: Taskboard ADR 0005.

## Context

Launcher starts at login and sits in the menu bar all day. If it dies, the global shortcut, the tray and the
CLI that Daybook calls die with it.

## Decision

The release profile keeps `panic = "unwind"`. Locks go through the poison-tolerant `MutexExt`/`RwLockExt`
(`src-tauri/src/lock.rs`); stores change a copy and swap it in after the save succeeds; a reachable
`unwrap()` becomes an `AppError`; long-running tasks (the socket listener, status polling of dev servers) are
supervised and restarted.

- Rejected: `panic = "abort"` (the template default); manual `catch_unwind` around commands.

## Consequences

- One bad project config or tool output fails one command with an error toast.
- Slightly larger binary than with abort.
