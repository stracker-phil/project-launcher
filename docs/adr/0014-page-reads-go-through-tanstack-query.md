# 0014. Page reads go through TanStack Query

## Status

Accepted (2026-11-11). Inherited from the family catalog (app-blueprint: `reads`, option TanStack Query); reference implementation: Taskboard ADR 0018.

## Context

The main window pulls most of what it shows: the project list for the current filter and search, one project's
resolved config, git and file stats for the inspector (three calls of very different speed), the settings.
Rust knows when data changes (a write, a config file edited on disk, a finished scan) but not which view is on
screen. The prototype already used React Query with per-feature keys.

## Decision

Page reads go through hooks in `src/queries/` built on TanStack Query: `staleTime: Infinity`, no retry, one
`queryClient` per window. Rust announces changes as events; `src/queries/useEvents.ts` maps each event to the
query keys it invalidates (`projects_changed` → `["projects"]`, `project_changed` → `["project", id]`,
`settings_changed` → `["settings"]`). Slow stats queries are enabled only while the inspector shows them.
zustand keeps UI-only state (selection, sidebar, search text) and sends writes. No polling except for live
status that has no event source (a dev server's state), and only while the window is visible.

- Rejected: Store + pushes (Rust would have to know what each window shows; Launcher pulls far more than it is
  pushed); hand-written version counters and re-reads (the pattern TimeTracker replaced); the prototype's
  re-read on every selection change (events make it unnecessary).

## Consequences

- A new read is a query hook plus the event that invalidates it; forgetting the mapping shows stale data,
  so `useEvents` has a test per event.
- Tests mock the API object (ADR 0002); query hooks need a `QueryClientProvider` in the test wrapper.
