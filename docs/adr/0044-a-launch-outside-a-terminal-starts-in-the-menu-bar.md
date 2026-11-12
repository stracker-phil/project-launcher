# 0044. A launch outside a terminal starts in the menu bar

## Status

Accepted (2026-11-12). Extends [0034](0034-a-menu-bar-app-that-keeps-running.md) (launch at login hidden).

## Context

Launcher lives in the menu bar; ⌃⌘L, the menu-bar item and the palette reach it. ADR 0034 started it hidden
only at login (`--hidden`); any other launch showed the main window. Once the window reopened where the owner
left it (ADR 0043), every start from Finder, Spotlight or a launcher app put a window in front, which the owner
doesn't want from a menu-bar app.

## Decision

A launch without a terminal (Launch Services: Finder, Spotlight, `open`, login) starts in the menu bar only.
`launcher` typed in a terminal still shows the window, unless given `--hidden`. `lib.rs` `starts_hidden` decides
from `--hidden` and whether stdin is a terminal. Opening the app again while it runs (`Reopen`), ⌃⌘L, the
menu-bar item and `launcher show` show the window as before. `scripts/install.sh` brings an open window back
after a release restart; `scripts/dev/launch.sh` shows the debug window for checks.

- Rejected: always hidden (typing `launcher` in a terminal is a request to see it); a setting (one more switch
  for a behavior with one right answer); remembering whether the window was open at quit (a forgotten open
  window would pop up at the next login).

## Consequences

- The first start from Finder shows only the menu-bar icon; its menu opens the window.
- `open -a Launcher` from a terminal also starts hidden, since Launch Services starts it.
