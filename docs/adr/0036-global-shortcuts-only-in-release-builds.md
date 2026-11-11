# 0036. Global shortcuts are registered only by release builds

## Status

Accepted (2026-11-11). Inherited from the family catalog (app-blueprint: `global-shortcuts-release-only`); reference implementation: Daybook ADR 0030.

## Context

The prototype toggled its main window with ⌃⌘L from anywhere. The debug build runs beside the installed app
and must not steal that key.

## Decision

Only release builds register global shortcuts (through `tauri-plugin-global-shortcut`), the login item and
the CLI link. The default is ⌃⌘L to show or hide the main window, rebindable in Settings → Shortcuts.
Registration pauses while a shortcut is being recorded. A taken shortcut is reported in Settings, not
retried. Debug builds show no global hints and run commands through the CLI, the palette or `js.sh`.

- Rejected: debug-only or shared shortcuts (Daybook ADR 0018, superseded); a fixed, unrebindable key.

## Consequences

- The installed app keeps its key while the debug app runs.
- The global shortcut can only be checked in a release build.
