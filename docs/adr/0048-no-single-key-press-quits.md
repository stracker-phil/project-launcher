# 0048. No single key press quits; ⌘Q and Esc hide the window

## Status

Accepted (2026-11-16). Extends [ADR 0034](0034-a-menu-bar-app-that-keeps-running.md), which keeps closing a window
from ending the app. Deviates from the family catalog (`closing-keeps-running`), whose apps quit on ⌘Q.

## Context

Launcher is summoned and dismissed many times a day. ⌘Q is the reflex for "go away", and Esc is the natural
key to dismiss a launcher. Quitting by accident drops the tray icon and the global shortcut until the app is
started again.

## Decision

⌘Q and Esc hide the window in front like ⌘W (page commands `main.hide`, `main.close`, `settings.hide`,
`settings.close`, running `window:close`). Esc first goes to a text field, a sheet, a popover or a menu; in the
main window the empty project search lets it through. Quit Launcher has no key, and Settings → Shortcuts cannot
give it one (`Item::takes_a_key`). It quits only from the Launcher menu, the tray menu or the palette.

- Rejected: ⌘Q quits (macOS default): one slip ends the menu-bar app.
- Rejected: a rebindable Quit key: any single key press could quit again.

## Consequences

- The only ways to quit are a menu click, the palette, or `launcher quit` from the CLI.
- A Quit key stored by an earlier version is ignored.
