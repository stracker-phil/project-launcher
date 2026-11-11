# 0003. Errors cross the boundary as a stable kind, never as parsed messages

## Status

Accepted (2026-11-11). Inherited from the family catalog (app-blueprint: `errors-as-kinds`); reference
implementation: Taskboard ADR 0003.

## Context

The prototype surfaced failures by stripping the IPC prefix from English error strings. Launching apps,
running scripts and reading user-owned config files fail in ordinary ways (folder gone, tool missing, bad
JSON) that the UI should handle differently.

## Decision

Every command returns `Result<T, AppError>`. `AppError` (thiserror, `src-tauri/src/error.rs`) serializes to
`{ kind, message, field? }` with kinds such as `notFound`, `invalid`, `config`, `tool`, `io`. The page
branches on `kind` only; `toAppError` in `src/api/index.ts` maps unknown kinds to `io`. The socket and
`launcher … --json` carry the same shape, so Daybook and scripts can branch on it too.

- Rejected: parsing message text; per-command ad-hoc error shapes.

## Consequences

- A field error (`field`) lands under the right Settings or inline editor field.
- Adding a kind is a Rust change plus regenerated types; wording can change without breaking callers.
