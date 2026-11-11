# 0007. App data and each project's config file each have exactly one write path

## Status

Accepted (2026-11-11). Inherited from the family catalog (app-blueprint: `one-write-path`); reference
implementation: Taskboard ADR 0019 (app data) and Daybook ADR 0015 (files the owner also edits).

## Context

Launcher writes two kinds of data: its own JSON stores (the project registry with launcher-only metadata,
settings and tags) and each project's hand-editable `.project-launcher.json`, which the owner also changes in
an editor and which may carry keys the app doesn't know.

## Decision

Every change to the app's own data goes through one write function of the data layer (`projects::Registry::
write`), which validates, saves, records the inverse for undo and emits one event. Every change to a project's
`.project-launcher.json` goes through one writer (`config::ConfigWriter::edit`): it serializes writes per
file, reads the current file fresh, applies the change, keeps unknown keys and the short form of built-ins,
writes a temp file and renames it, and retries when the file changed underneath. No other code writes either.

- Rejected: handlers writing files directly; caching the parsed config and writing it back (loses the owner's
  concurrent edits and unknown keys).

## Consequences

- Validation, events and undo live in one place; the CLI and the window behave the same.
- A hand edit between read and write is never overwritten silently.
