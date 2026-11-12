# 0043. The window frame and sidebar choice are remembered outside undo

## Status

Accepted (2026-11-12). Amends [0038](0038-project-details-live-in-the-project-file.md)'s list of what Launcher's
data folder keeps; [0007](0007-one-write-path-for-app-data-and-project-files.md) and
[0015](0015-writes-are-operations-with-inverses-for-undo.md) keep covering every data change.

## Context

Launcher always started on All Projects at the default window size from `tauri.conf.json`. The owner wants it to
come back as it was left: the sidebar page (a tag, Starred, Archived …) and the main window's size and place.
Both change often and are about the window, not about the projects or the settings. Undo exists to take back
changes to the owner's data; a window move or a sidebar click is not one.

## Decision

- A third app file, `view.json`, holds `{ "mainFrame": { x, y, width, height } | null, "filter": <Filter> | null }`
  through `JsonStore` (atomic, 0600, repaired per part on load; an invalid part is `null`). It is written straight
  through its store, outside the one writer and the undo stack, and announces nothing.
- Rust tracks the main window's frame (logical points: outer position, inner size) on every move and resize,
  keeps it in memory and saves it at most once a second, plus when the window hides and the app quits. A
  minimized or full-screen window keeps the frame it had. At start, before the window first shows, the saved
  frame is applied when at least half of it lies on a connected screen; otherwise the default stays. In debug
  builds `LAUNCHER_WINDOW_FRAME` wins and turns saving off, so scripts never move the owner's window.
- The page reports each sidebar filter change (`set_view_filter`) and asks for the filter at start
  (`view_filter`). Rust decides whether it still applies: a tag nobody has any more, or Untagged while its row is
  hidden, falls back to All.

Rejected:

- `settings.json` through the undoable writer: ⌘Z would take back a window move or a sidebar click before the
  edit the owner meant, and every move would rebuild the menus as a settings change.
- `tauri-plugin-window-state`: a dependency for one window, with its own file and rules, and the filter needs a
  home anyway.
- `localStorage`: page-side and per webview, out of Rust's reach (ADR 0001) and gone with the webview's data.

## Consequences

- The data folder holds `settings.json`, `projects.json` and `view.json`; Settings → Privacy & Data and the
  README say so.
- The debug build has its own `view.json` in its own data folder.
- The page starts on All for a moment and switches when Rust's answer arrives; a filter the owner picks first wins.
