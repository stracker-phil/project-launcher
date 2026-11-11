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
| `app:settings` | Settings… | Opens the Settings window where it was; ⌘, |
| `app:settings:<page>` | Settings Page | Hidden. Opens Settings at a page or row id (`status`, `about`, `cli`, …); an unknown id shows the first page |
| `page:<command>` | Window Action | Hidden. Shows the main window and emits `page_command` with `<command>` |
| `window:close` | Close Window | Hidden. Hides the window in front (⌘W) |
| `edit:undo` | Undo | ⌘Z. With the main or Settings window focused it emits `edit_request` `"undo"` to it; the page undoes the typing of a focused text field (`document.execCommand`), else calls `undo_change`. Without a focused window it undoes the last data change directly |
| `edit:redo` | Redo | ⇧⌘Z; as `edit:undo`, with `"redo"` and `redo_change` |
| `app:logs` | Open Log Folder | Help menu |
| `app:data` | Show Data Folder | Help menu; the config folder with `settings.json` and `projects.json` (the list of known project files) |
| `app:quit` | Quit Launcher | Quits; tray and ⌘Q |
| `app:reload` | Reload | Debug builds only; reloads the focused window (⌥⌘R) |

## Page commands

The page's own commands live in `src/commands/` (`registerCommands`, `runPageCommand`). Menus reach them as
`page:<id>`; a page command with a `defaultBinding` (or a key the owner gave it) also runs from its key while
the window has focus. Bare keys (no ⌘ or ⌃) never fire in text fields.

| Id | Title | Key (menu) |
|---|---|---|
| `view.toggleSidebar` | Toggle Sidebar | ⌃⌘S |
| `view.toggleInspector` | Toggle Inspector | ⌃⌘I |
| `palette.open` | Command Palette | ⌘K; opens the palette, or closes it when open |

The Settings window has its own (`settingsCommands.ts`):

| Id | Title | Key |
|---|---|---|
| `settings.close` | Close Settings | ⎋ (not while a text field or sheet has focus); runs `window:close` |
| `settings.search` | Search Settings | ⌘F |

## The palette

The command palette (`src/palette/`, [ADR 0031](adr/0031-the-palette-runs-the-menus.md)) lists, when it opens:

- every enabled menu-bar item from `MENUS` except Command Palette… itself, in menu order, from the Tauri command
  `menu_commands` (`src-tauri/src/menu/palette.rs`: `MenuCommand` with menu title, label, command id and the key
  as the menu shows it, `⌃⌘S`). A `page:<id>` item runs the page command directly (and is left out when that
  can't run now); any other item runs through `api.runCommand`;
- the page commands with a title that no menu item runs (matched on `page:<id>`), under "Window";
- the rows of menus a component offers while on screen (`usePaletteMenu`), as "Title: Item › Subitem";
- each Settings page as "Settings: <page>", which runs `app:settings:<id>`.

A failure shows as an error toast.

## Menus

`src-tauri/src/menu/items.rs` places commands; an item shows only when the bus knows its command, and a
group without one is dropped. Plain-letter keys never go on menu items.

| Menu | Items |
|---|---|
| Launcher | About · Settings… ⌘, · Services · Hide, Hide Others, Show All · Quit Launcher ⌘Q |
| File | Close Window ⌘W |
| Edit | Undo ⌘Z, Redo ⇧⌘Z (`edit:undo`, `edit:redo`, replacing macOS's own) · macOS's Cut, Copy, Paste, Select All |
| View | Toggle Sidebar ⌃⌘S, Toggle Inspector ⌃⌘I · Command Palette… ⌘K (`page:palette.open`), Reload ⌥⌘R (debug) · Enter Full Screen |
| Window | Minimize, Zoom · Bring All to Front |
| Help | Open Log Folder, Show Data Folder |

The keys above are the defaults. The owner changes them in Settings → Shortcuts; `settings.json` `shortcuts`
stores them by command id in the page's notation (`Mod+Shift+K`): a menu item under its command
(`page:palette.open`, `app:quit`), a page command under its id (`settings.search`). `menu/keys.rs` converts
between that and Tauri's `CmdOrCtrl+Shift+K` (`Combo`); a menu item's key is the override, else its default
(`effective_accelerator`; an override the menu can't take keeps the default). Every settings change, its undo
and redo included, rebuilds the menu bar (`menu::refresh`), and `menu_commands` reports the keys in effect.

| Tauri command | Does |
|---|---|
| `menu_shortcuts` | Every menu item of a known command, enabled or not, with `defaultKey` and `key` (in effect) in the page's notation (`MenuShortcut`) |
| `set_shortcut(command, key)` | Stores the owner's key (`null`: none); a menu item's key needs ⌘ or ⌃ and is stored normalized (`menu::shortcuts::checked`), else `InvalidField` `shortcut` |
| `reassign_shortcut(command, key, from)` | Gives `command` the key and leaves `from` without one, as one undo step ("Reassign Shortcut") |
| `reset_shortcut(command, from?)` | Back to the default key; `from`, which has that key now, is left without one in the same undo step ("Reset Shortcut") |
| `pause_menu_keys(paused)` | Rebuilds the menu bar without (or again with) its keys while Settings → Shortcuts records one |

The tray menu's items are `app:show` and `app:quit`; Tauri hands tray clicks to the same handler
(`menu::clicked`).

## Undo and redo

The menu bar's ⌘Z never reaches a text field, so the Edit menu's Undo and Redo ask the focused window
(`edit_request`) and its page decides (`src/commands/useEditRequests.ts`): a focused text field undoes its typing,
anything else undoes the last change of the data (`api.undo` / `api.redo` → `undo_change` / `redo_change`, which
return the change's label). The history is the app run's last 100 changes ([ADR 0015](adr/0015-writes-are-operations-with-inverses-for-undo.md)).

## Developer commands

Debug builds only.

| Command | Does |
|---|---|
| `launcher dev seed --yes` | Replaces the debug build's projects, project folders and settings with the demo seed and clears the undo history; refused without `--yes` |
