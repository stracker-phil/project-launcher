# 0002. Only the API files talk to Tauri; the rest of the page imports `api`

## Status

Accepted (2026-11-11). Inherited from the family catalog (app-blueprint: `ipc-in-api-files`); reference
implementation: Taskboard ADR 0002.

## Context

The prototype called `invoke(channel, …)` from one wrapper but subscribed to notifications in many
components, so tests had to stub the Glaze bridge piecemeal.

## Decision

Only `src/api/index.ts` and `src/<module>/api.ts` import `@tauri-apps/*`. Components, hooks and stores import
the `api` object. UI tests mock exactly that object through `createApiMock()` (`src/test/apiMock.ts`), and
`src/test/boundary.test.ts` fails on any other `@tauri-apps` import.

- Rejected: `invoke`/`listen` scattered through components; one shared API file for every module (a feature
  could no longer be removed as a folder).

## Consequences

- The contract between page and Rust is visible in one place per module and mocked in one place.
- A new command needs a line in the module's `api.ts` before any component can use it.
