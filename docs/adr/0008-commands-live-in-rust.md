# 0008. App actions are commands of one Rust bus

## Status

Accepted (2026-11-11). Inherited from the family catalog (app-blueprint: `commands-in-rust`); reference
implementation: Daybook ADR 0017.

## Context

The prototype already routed every action through one command id, but the page owned the table: it built
the menu model and handled most ids, so the tray and the CLI had to wake the page (and queue commands until it
loaded) to open a project from the menu bar.

## Decision

App-level actions are commands of one Rust `CommandBus<H: Host>` (`src-tauri/src/bus/`): colon ids with
prefix families (`project:open`, `project:open:<id>`, `project:script:<n>`, `filter:tag:<name>`,
`app:settings:<page>`), each with title, help, enabled/checked checks and a handler. The menu bar, the tray
menu, the global shortcut, the floating palette, the CLI and links run them by id. The page keeps a registry
only for in-window keys (`page:<id>` → `page_command` event), rebindable in Settings → Shortcuts. Links run an
allowlist of command ids. The bus is testable without Tauri.

- Rejected: the prototype's page-owned command table; a handler wired per surface; moving page logic into Rust.
- Rejected: the smaller catalog variant; Launcher's commands take arguments (project id, tag, script index).

## Consequences

- Opening a project or running a script from the tray works with the window hidden and never loaded.
- `docs/commands.md` lists every id; a new action is one bus entry and then appears everywhere.
