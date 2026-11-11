# 0004. IPC types are generated from Rust with ts-rs

## Status

Accepted (2026-11-11). Inherited from the family catalog (app-blueprint: `generated-ipc-types`); reference
implementation: Taskboard ADR 0004.

## Context

The prototype kept `main/services/types.ts` and `renderer/lib/types.ts` in sync by hand, and its guide had to
remind every change to edit both.

## Decision

Every IPC type derives `ts_rs::TS` and exports camelCase into `src/api/types.gen.ts` (one file per module).
`bun run types` regenerates them; `bun run lint` fails when the generated file differs from the committed one.
`.cargo/config.toml` sets `TS_RS_EXPORT_DIR` and `TS_RS_LARGE_INT = "number"`. A constant mirrored by hand
gets a Rust test that compares both copies.

- Rejected: hand-written TypeScript mirrors; specta/tauri-specta; JSON Schema.

## Consequences

- A renamed field breaks the TypeScript build instead of a running window.
- Tests and lint must not run at the same time: both regenerate `types.gen.ts`.
