# 0023. Debug builds carry hooks for driving the app without focus

## Status

Accepted (2026-11-11). Inherited from the family catalog (app-blueprint: `debug-hooks`); reference implementation: Taskboard ADR 0011.

## Context

Agents and scripts verify each step in the running debug app: they place windows, read state, run commands and
take screenshots while the owner keeps working. AppleScript clicks and `open` steal focus and break that.

## Decision

Debug-only hooks (`#[cfg(debug_assertions)]`, `src-tauri/src/dev/`): `LAUNCHER_WINDOW_FRAME="x,y,w,h"`,
`LAUNCHER_BACKGROUND=1` (no focus), `eval` over the socket (`@<label>` targets a window, e.g. `@palette`), the
demo-source switches (ADR 0022); the zustand store on `window.launcher` with `VITE_DEV_HOOKS=1`. Scripts drive
the running debug app without focus: `scripts/dev/launch.sh` (next to the terminal, refuses a stale bundle),
`js.sh`, `screenshot.sh [--overlay]` (debug window only, also on another Space; `--overlay` for the palette
panel), `e2e.sh` (fails on ≥1px drift of sidebar, list, first row or a clicked control), `screenshot-looks.sh`
and a README-screenshots script. Never AppleScript clicks or keystrokes; never `open`.

- Rejected: driving the app through Accessibility or AppleScript (steals focus, flaky); hooks in release
  builds.

## Consequences

- Every step can be checked end to end without interrupting the owner.
- The global show/hide shortcut is release-only, so debug runs go through the CLI, the palette or `js.sh`.
