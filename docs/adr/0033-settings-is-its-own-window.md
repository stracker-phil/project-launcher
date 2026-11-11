# 0033. Settings is a window of its own in System Settings layout

## Status

Accepted (2026-11-11). Inherited from the family catalog (app-blueprint: `settings-window`); reference implementation: Taskboard ADR 0010.

## Context

The prototype had a fixed 560×560 Settings window with sections for tools (editor, git client, terminal),
appearance and the command line. More pages will come (shortcuts, status), and it should stay open beside the
main window.

## Decision

Settings is the `settings` window (~880×720, min 760×560, not maximizable, created hidden, overlay title
bar) with a 190px page list, a drag strip and a search field (page, group and section titles; ⌘F; ↵ opens
the first hit). Changes apply at once; fields commit on blur or ↵ and show Rust's field error below. Esc, ⌘W
and close hide it; it reopens on the same page. Rust broadcasts `settings_changed` to every window. Pages
register in `src/registry/settingsPages.ts`: Tools, Appearance, Shortcuts, Status, About, Privacy & Data.

- Rejected: a modal sheet; Settings as a view of the main window; the prototype's fixed small window.

## Consequences

- Settings is searchable and can stay open while the owner works.
- Opening Settings shows the Dock icon ([0034](0034-a-menu-bar-app-that-keeps-running.md)).
