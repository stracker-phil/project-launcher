# 0031. The command palette runs the menus

## Status

Accepted (2026-11-11). Inherited from the family catalog (app-blueprint: `palette-runs-the-menus`); reference implementation: Taskboard ADR 0043.

## Context

The palette is Launcher's fastest path: find a project, run one of its actions. In the prototype it fetched
the page's menu model, so palette and menu bar agreed; that property is worth keeping with the Rust bus.

## Decision

⌘K (and the tray) opens the palette, which lists every enabled menu-bar item from the same `MENUS` table
(`menu_commands`), plus menus components offer while on screen (`usePaletteMenu`) and Settings pages from
the registry. Ranking on the page: label starts with → a word starts with → every word → letters in order.
Prefixes: `>` commands, and from the prototype `:` projects, `#` library (filters and tags), `/` actions of
the current project. Page commands are snapshotted when it opens. The palette lives in its own floating panel
window ([0037](0037-the-palette-is-a-floating-panel-on-a-native-window-layer.md)).

- Rejected: a separate palette list; page commands listed in Rust.

## Consequences

- A new menu item appears in the palette with its shortcut, with no extra code.
- The panel window needs the menu snapshot without the main page being visible.
