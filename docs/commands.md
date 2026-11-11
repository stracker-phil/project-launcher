# Commands

Every app action is a command of one Rust bus (`src-tauri/src/bus/`, [ADR 0008](adr/0008-commands-live-in-rust.md)). The menu bar, the
tray and the page (`api.runCommand` → `run_command`) run them by id. A command is registered once in
`bus::register` (or a feature's own `register`) and then appears in every menu that places it.

## The bus

- `Command::new(id, title)` with `.help()`, `.hidden()` (left out of lists), `.quits()`, `.enabled_when()`,
  then `.run(async)` or `.run_now(sync)`; `.family("<arg>")` makes the id a prefix that takes the rest of the
  id as its argument.
- An exact id wins over a family; among families the longest prefix wins. Registering an id again replaces it.
- `run_command` returns the failure to the page; menu and tray clicks (`bus::spawn`) log it.

## App commands

| Id | Title | Notes |
|---|---|---|
| `app:show` | Open Launcher | Shows the main window; tray |
| `page:<command>` | Window Action | Hidden. Shows the main window and emits `page_command` with `<command>` |
| `window:close` | Close Window | Hidden. Hides the window in front (⌘W) |
| `app:logs` | Open Log Folder | Help menu |
| `app:data` | Show Data Folder | Help menu; the config folder with `settings.json` and `projects.json` |
| `app:quit` | Quit Launcher | Quits; tray and ⌘Q |
| `app:reload` | Reload | Debug builds only; reloads the focused window (⌥⌘R) |

## Page commands

The page's own commands live in `src/commands/` (`registerCommands`, `runPageCommand`). Menus reach them as
`page:<id>`; a page command with a `defaultBinding` also runs from its key while the window has focus. Bare
keys (no ⌘ or ⌃) never fire in text fields.

| Id | Title | Key (menu) |
|---|---|---|
| `view.toggleSidebar` | Toggle Sidebar | ⌃⌘S |
| `view.toggleInspector` | Toggle Inspector | ⌃⌘I |

## Menus

`src-tauri/src/menu/items.rs` places commands; an item shows only when the bus knows its command, and a
group without one is dropped. Plain-letter keys never go on menu items.

| Menu | Items |
|---|---|
| Launcher | About · Settings… ⌘, (once registered) · Services · Hide, Hide Others, Show All · Quit Launcher ⌘Q |
| File | Close Window ⌘W |
| Edit | macOS's Undo, Redo, Cut, Copy, Paste, Select All |
| View | Toggle Sidebar ⌃⌘S, Toggle Inspector ⌃⌘I · Command Palette… ⌘K (`page:palette.open`), Reload ⌥⌘R (debug) · Enter Full Screen |
| Window | Minimize, Zoom · Bring All to Front |
| Help | Open Log Folder, Show Data Folder |

The tray menu's items are `app:show` and `app:quit`; Tauri hands tray clicks to the same handler
(`menu::clicked`).
