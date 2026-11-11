# 0035. Launcher has no status bar

## Status

Accepted (2026-11-11). Inherited from the family catalog (app-blueprint: `status-bar`, Choice: no status bar); rejected option's reference: Taskboard ADR 0060.

## Context

Launcher has few app-wide facts: no sync, no timer. Facts like a project's git state, its dev server or
Docker status belong to the selected project. The prototype had no status bar.

## Decision

No status bar. View facts stay with the view (the detail pane, the inspector, list dots); background work
such as starting a dev server shows on the control that started it.

- Rejected: a status bar for app facts (Taskboard ADR 0060): it would stay mostly empty.

## Consequences

- One less row of chrome; the content panel reaches the window bottom.
- If an app-wide fact appears later (e.g. a folder scan), a new ADR revisits this.
