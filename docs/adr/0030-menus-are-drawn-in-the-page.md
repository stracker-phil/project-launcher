# 0030. Context menus and pop-up buttons are drawn in the page

## Status

Accepted (2026-11-11). Inherited from the family catalog (app-blueprint: `menus-in-the-page`); reference implementation: Taskboard ADR 0034.

## Context

The prototype used native NSMenus for context menus, the ⋯ menu and the tag picker. They cannot show the
project color tiles or palette colors, need an IPC round trip per menu, and are hard to test.

## Decision

One in-app menu component draws every context menu and pop-up button: a store slice holds the open menu,
`MenuHost` (once per window) draws it, `openMenu`/`openContextMenu` open it. It behaves like a macOS menu:
placement at the mouse or under the button; Esc, outside click, scroll and blur close it; arrow and letter
keys; submenus (Move to Tag); focus return; ARIA roles. A `PopUpButton` replaces `<select>`. The ⋯ menu and
the right-click menu of a project are built from one item list. The menu bar and the tray menu stay native.

- Rejected: native menus as in the prototype (no tiles or palette colors, IPC per menu, untestable).

## Consequences

- Menus follow the palettes and are covered by UI tests.
- Keyboard behavior must be kept macOS-like by the component.
