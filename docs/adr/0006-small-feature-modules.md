# 0006. Code is organised in small feature modules with pure cores

## Status

Accepted (2026-11-11). Inherited from the family catalog (app-blueprint: `small-feature-modules`); reference
implementation: Taskboard ADR 0014.

## Context

The prototype already split backend services from IPC handlers and kept one file per window and settings
section, but several files grew past 500 lines (`home-view.tsx`, `action-sections.tsx`, `stage-hooks.ts`).

## Decision

Folders by feature: `src-tauri/src/<feature>/` (e.g. `projects/`, `config/`, `launch/`, `tags/`,
`environment/`, `stats/`, `databee/`) with pure logic, a thin `commands.rs` and tests, and a matching
`src/<feature>/`. The core never imports a feature; features register in `handlers.rs` and `src/modules.ts`.
I/O stays at the edges (processes, files, git); the cores are pure. A trait only when a second implementation
exists today (a fake counts); macOS-only code (objc2) sits behind one module. Files over ~400 lines of
production code are split in the same step.

- Rejected: layering by type (all commands in one place); a trait per module.

## Consequences

- A feature such as dev environments or project stats can be removed as a folder plus one line each.
- Shell and git access are behind small edges that tests replace with fakes.
