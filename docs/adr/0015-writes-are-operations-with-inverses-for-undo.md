# 0015. Writes are operations with inverses, so every change can be undone

## Status

Accepted (2026-11-11). Inherited from the family catalog (app-blueprint: `undo`, option ops with inverses); reference implementation: TimeTracker ADR 0010.

## Context

All of Launcher's data is local: the registry, tags and launcher-only metadata in the config dir, and the
launch config in each project's `.project-launcher.json`. The prototype removed a project at once and offered
Undo in a toast. Edits happen inline (rename, icon and colour, tag, notes, star, archive, actions and scripts),
from the menu bar, the palette and the CLI, so undo has to work the same from every surface.

## Decision

Every write is an `Op` in `src-tauri/src/projects/ops.rs` with a computed inverse, applied through the one
write path (ADR 0007): add or remove a project (remove keeps the full entry so the inverse restores it), rename,
set icon and colour, set or clear the tag, notes, star, archive, tag create/rename/delete, and edits to a
`.project-launcher.json` (the inverse holds the file's previous content and is refused if the file changed
on disk since). `AppState` keeps a bounded undo/redo stack (100 entries) for the app run; it is not persisted.
⌘Z and ⇧⌘Z run `edit:undo` and `edit:redo`; CLI writes go through ops too, so they are undoable in the app.
Each undo announces the same events as the original write.

- Rejected: plan, review, apply (heavy for edits this small; nothing here is destructive enough to need a
  review); "No undo, Change back to X" (meant for writes to a remote service; it loses the prototype's Undo
  after removing a project).

## Consequences

- Removing a project needs no confirmation sheet; Undo brings it back with its metadata.
- Every new kind of write is a new `Op` with an inverse and a round-trip test (apply, invert, compare).
- Undo ends when the app quits; launching an app or running a script is not a write and is not undoable.
