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
| `page:<command>` | Window Action | Hidden. Emits `page_command` with `<command>` to the main window, showing it first unless the command acts on the current project without it (`window::IN_BACKGROUND`: terminal, reveal, copy path, edit file, star, archive, lock; and `action:<list>:<index>`, `tag:<json>`). The page also takes `filter:<json>` (shows a sidebar filter), `action:<list>:<index>` (launches the current project's action) and `tag:<json>` (moves the current project to a tag, `null` for none, as the right-click's Move to Tag; not while it is locked; `main/pageRequests.ts`) |
| `app:palette` | Command Palette… | ⌘K; opens or closes the palette panel; tray |
| `app:toggle` | Show or Hide Launcher | The global shortcut ⌃⌘L (release builds only, ADR 0036); hides the main window while it is in front, else shows it; rebindable under Settings → Shortcuts → Global |
| `project:show:<id>` | Show Project | Hidden. Shows the main window with the project selected (`select_project` with `{ id, filter }`: Archived for an archived project, else All Projects); palette and tray |
| `project:launch:<id>:<list>:<index>` | Launch Action | Hidden. Runs a project's action in Rust; the tray's rows |
| `project:reveal:<id>` | Reveal Project in Finder | Hidden. The tray's row |
| `dev:<step>:<id>` | Dev Environment | Hidden. A project's dev environment (`devenv/bus.rs`): `docker-start`, `docker-stop`, `docker-restart`, `server-start`, `server-stop`, `server-restart` run the same step as the detail's buttons (`dev_action`: on a thread of its own, `dev_changed`, `dev_failed`); `open-site` opens the running server's URL in the browser (refused while it doesn't run). An unknown project is `NotFound`. The palette; `launcher command dev:server-start:<id>` |
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
| `app:palette` (bus) | Command Palette | ⌘K; opens the palette, or closes it when open |
| `project.new` | New Project… | ⌘N; the folder panel, several folders at once (`add_project_folders`, one undo step) |
| `tag.new` | New Tag… | ⇧⌘N; a field at the end of the sidebar's tags |
| `view.search` | Search Projects | ⌘F; focuses the list's search field |
| `library.all`, `library.starred`, `library.archived` | All Projects, Starred, Archived | ⌘1, ⌘2, ⌘0 |
| `project.rename`, `project.icon`, `project.notes` | Rename Project, Change Icon & Color…, Edit Note | ⌘E for Rename; the selected project's field in edit mode; off while it is locked |
| `project.newTag` | New Tag for Project… | The project's tag field in edit mode (the right-click's New Tag…); off while it is locked; palette only |
| `project.star`, `project.archive` | Star Project, Archive Project | Toggle; starring unarchives and archiving unstars, in one undo step |
| `project.lock` | Lock Project Details | ⇧⌘L; toggles `"locked"` in the project file |
| `project.terminal`, `project.reveal`, `project.copyPath`, `project.editFile` | Open Terminal Here, Reveal in Finder, Copy Path, Edit Project File… | ⌥⌘C for Copy Path; a shell in the folder with the project's env, the folder in Finder, the absolute path on the clipboard, `.project-launcher.json` in the config editor (else the project's or default editor, else the text editor) |
| `project.database` | Open Database | ⇧⌘D; the project's database in DataBee (`open_database`); a DDEV or wp-env database whose server doesn't run is started first |
| `project.remove` | Remove from List | The toast offers Undo; the folder and its file stay |

The library's and project's commands live in `src/main/libraryCommands.ts`; they act on the project the detail
shows (`useLibrary` `current`). A row's right-click opens the same choices (`src/main/projectMenu.ts`).

The Settings window has its own (`settingsCommands.ts`):

| Id | Title | Key |
|---|---|---|
| `settings.close` | Close Settings | ⎋ (not while a text field or sheet has focus); runs `window:close` |
| `settings.search` | Search Settings | ⌘F |

## The palette

The command palette is its own window, a floating non-activating panel (`palette`, [ADR 0037](adr/0037-the-palette-is-a-floating-panel-on-a-native-window-layer.md)):
`window/palette.rs` builds it hidden at startup and turns it into an `NSPanel` (`window/native.rs`). `app:palette`
(⌘K, the tray) opens it centred over the main window while that shows, else on the pointer's screen, a fifth of the
way down (`place`). It opens invisible and emits `palette_opened` with the project the main window shows; the page
(`src/windows/palette/`) remounts its panel and calls `palette_shown`, which makes it visible. ⎋, ⌘K, a chosen
row (`close_palette`) or a click elsewhere (after a 400 ms settle) closes it; a second toggle within 250 ms is
ignored. The app in front stays in front.

It lists ([ADR 0031](adr/0031-the-palette-runs-the-menus.md), `src/palette/sources.ts`):

- every project (archived last; `project:show:<id>`), prefix `:`;
- the sidebar's filters and tags with their counts (`page:filter:<json>`), prefix `#`;
- under the project's name, prefix `/`: the current project's actions (`page:action:<list>:<index>`); its dev
  environment's steps that apply now (`dev_commands`, `devenv/bus.rs` `palette_items`): Start Docker while it is
  stopped, Restart Docker and Stop Docker while it runs; Start <server> ("DDEV", "wp-env") while the server is
  stopped (Docker too, as the server's start starts Docker first), Restart, Stop and Open Site (with a URL) while
  it runs; nothing for a part that is busy or a server that isn't set up; then the Project menu's items as they
  apply to it (`menu_commands` with `current`, `menu/project_items.rs` `for_project`): Rename, Icon, Note and the
  tag items only while it is unlocked, Open Database only with a database DataBee can open, Star/Unstar,
  Archive/Unarchive, Lock/Unlock by its state, and after Lock "Move to Tag: <tag>" for every other tag
  (`page:tag:<json>`), "Move to Tag: No Tag" while it has one and "New Tag for Project…" (`page:project.newTag`).
  The menu bar's Project menu keeps its fixed labels;
- every other enabled menu-bar item from `menu_commands` (`src-tauri/src/menu/palette.rs`) and each Settings page
  as "Settings: <page>" (`app:settings:<id>`), prefix `>`.

Everything runs through `api.runCommand`, so the main window's page does the work and shows its toasts.

## Menus

`src-tauri/src/menu/items.rs` places commands; an item shows only when the bus knows its command, and a
group without one is dropped. Plain-letter keys never go on menu items.

| Menu | Items |
|---|---|
| Launcher | About · Settings… ⌘, · Services · Hide, Hide Others, Show All · Quit Launcher ⌘Q |
| File | New Project… ⌘N, New Tag… ⇧⌘N · Close Window ⌘W |
| Edit | Undo ⌘Z, Redo ⇧⌘Z (`edit:undo`, `edit:redo`, replacing macOS's own) · macOS's Cut, Copy, Paste, Select All |
| View | Toggle Sidebar ⌃⌘S, Toggle Inspector ⌃⌘I · Search Projects ⌘F, Command Palette… ⌘K (`app:palette`), Reload ⌥⌘R (debug) · Enter Full Screen |
| Library | All Projects ⌘1, Starred ⌘2 · Archived ⌘0 |
| Project | Rename Project ⌘E, Change Icon & Color…, Edit Note · Star Project, Archive Project, Lock Project Details ⇧⌘L · Open Terminal Here, Reveal in Finder, Copy Path ⌥⌘C, Open Database ⇧⌘D, Edit Project File… · Remove from List |
| Window | Minimize, Zoom · Bring All to Front |
| Help | Open Log Folder, Show Data Folder |

The keys above are the defaults. The owner changes them in Settings → Shortcuts; `settings.json` `shortcuts`
stores them by command id in the page's notation (`Mod+Shift+K`): a menu item under its command
(`app:palette`, `app:quit`), a page command under its id (`settings.search`). `menu/keys.rs` converts
between that and Tauri's `CmdOrCtrl+Shift+K` (`Combo`); a menu item's key is the override, else its default
(`effective_accelerator`; an override the menu can't take keeps the default). Every settings change, its undo
and redo included, calls `menu::refresh`, which rebuilds the menu bar only when the keys or their pause
changed (`MenuKeys` remembers what the shown bar was built with), and `menu_commands` reports the keys in effect.

| Tauri command | Does |
|---|---|
| `menu_shortcuts` | Every menu item of a known command, enabled or not, with `defaultKey` and `key` (in effect) in the page's notation (`MenuShortcut`) |
| `set_shortcut(command, key)` | Stores the owner's key (`null`: none); a menu item's key needs ⌘ or ⌃ and is stored normalized (`menu::shortcuts::checked`), else `InvalidField` `shortcut` |
| `reassign_shortcut(command, key, from)` | Gives `command` the key and leaves `from` without one, as one undo step ("Reassign Shortcut") |
| `reset_shortcut(command, from?)` | Back to the default key; `from`, which has that key now, is left without one in the same undo step ("Reset Shortcut") |
| `pause_menu_keys(paused)` | Rebuilds the menu bar without (or again with) its keys while Settings → Shortcuts records one |

The tray menu (`tray.rs`): Open Launcher, Command Palette…, Recent Projects ▸ (the ten last opened, not archived,
not the current one, ⌥1…⌥0; `project:show:<id>`), the current project's actions with their keys and Reveal in
Finder (`project:launch:…`, `project:reveal:<id>`), Settings… and Quit. `tray::refresh` runs when the projects or the
settings change (`announce`) and when the main window reports another current project (`set_current_project`);
it rebuilds the menu only when its rows differ from the shown ones.
Tauri hands tray clicks to the same handler (`menu::clicked`).

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

## Launching

An action's key (its `shortcut`, or a built-in's default such as ⌘O for Edit Code) launches it while the main
window shows its project and no text field, menu, sheet or palette has the keys
(`src/main/detail/useActionShortcuts.ts`). A menu item with the same key wins.

## Focus

The project list keeps the keys whenever nothing else needs them (`src/lib/focus.ts`): ↑↓ move its selection
from anywhere in the main window unless a text field, menu, popover or sheet has the keys
(`ProjectList.tsx`); a field that closes (rename, note, tag, link), a popover or sheet whose opener is gone, a
project picked in the palette and a window that opens again hand the keys back to it. A popover takes the
keys when it opens (its chosen item first) and gives them back to its anchor. A field that loses the keys only
because the app does (another app, the palette) stays open. Commands that act on the window in front (Close
Window, Reload) aren't in the palette, which would be that window.

| Tauri command | Does |
|---|---|
| `launch_action(id, source)` | Runs the action at `source` (`run/mod.rs` `Runner::launch`): an app on the folder or its arguments (a CLI on the login PATH directly, else `open -a`), a link, a terminal command, or a script in a login `zsh` with a 120 s timeout. Returns `Outcome`: `opened` with `Launched` (`launched`, `focused`, `needsPermission`, `activated`) or `ran` with the `ScriptResult` |
| `open_terminal(id)` | A shell in the folder with the project's env |
| `reveal_project(id)`, `copy_project_path(id)`, `edit_project_file(id)` | The folder in Finder, its path on the clipboard, the project file in an editor |
| `project_status(id)` | Runs the state providers: the states to show, the actions their `hiddenStates` hide now, the terminal commands still running (`ProjectStatus`) |

Everything but copying the path records the project as opened (`lastOpenedAt`) and announces `projects_changed`.

## Dev environment

| Tauri command | Does |
|---|---|
| `dev_running()` | The ids of the projects whose DDEV or wp-env server runs, asked in one go (`ddev list`, `docker ps`); none while Docker is stopped. The list's green dots |
| `dev_commands(id)` | The `dev:` commands that apply to the project now, as `MenuCommand`s, for the palette; from a status read within 15 s (`DevEnv::recent_status`, cleared by every step), so the palette answers at once |
| `dev_status(id)` | `DevStatus`: Docker (running, the step under way, the manager's name) for a project that uses it, the DDEV or wp-env server (state, URL, step under way; `notConfigured` with a hint when the folder lacks its config) and the database (`DatabaseInfo`) |
| `dev_action(id, action)` | Starts, stops or restarts Docker or the server on a thread of its own (`DevAction`); Docker's stop first runs `ddev poweroff` while DDEV projects run. `dev_changed` follows each change, `dev_failed` (`{ projectId, label, output }`) a failed step |
| `open_database(id)` | Opens the database in DataBee through its CLI (`databee`, the password in `DATABEE_PASSWORD`); `starting` when the server is started first, then DataBee opens or `dev_failed` reports why not |
| `set_docker_manager(manager)` | Docker Desktop, OrbStack, Colima or Custom (its start and stop commands through `set_app_setting`) |

Launching an action or a terminal starts the project's environment first, once per session (`DevEnv::prepare`), so
an environment stopped by hand stays stopped.

## Inspector

| Tauri command | Does |
|---|---|
| `quick_stats(id, refresh)` | `QuickStats`: the kind line ("WordPress plugin · v3.1.2"), requirements, the dependency counts ("43 npm · 20 composer"), package scripts and git (`None` outside a repository) |
| `file_stats(id, refresh)` | `FileStats`: files (`truncated` past 50 000), source size, size on disk, lines, languages (the six largest, the rest as Other) and the five files edited last |
| `run_package_script(id, manager, name)` | Runs `npm run <name>` (or pnpm, yarn, bun, `composer run-script`) in a terminal; one still running there comes to the front |
| `open_project_file(id, path)` | Opens a file of the project (a relative path inside its folder) in its editor, else the default editor, else its default app |

Both stats stay cached for five minutes; `refresh` reads again (the inspector's Refresh button).

## Command line

The same binary is the command line ([ADR 0009](adr/0009-the-cli-asks-the-running-app.md)); each subcommand asks the running app over its socket (`cli/`,
`remote/automation.rs`, the name matching in `automation/`). `--json` prints the answer as it came.

| Subcommand | Does |
|---|---|
| `launcher` / `--hidden` | Starts Launcher or brings it forward; `--hidden`, or a start without a terminal (Finder, Spotlight, login), starts it in the menu bar (`starts_hidden`, ADR 0044) |
| `show`, `focus`, `hide`, `quit`, `status` | The window and the app |
| `list [--json]` | Every project: NAME, TAG, PATH (`~` for home); JSON entries `{ id, name, path, tag, starred, archived }` with an absolute path. Daybook calls `launcher list --json`; the fields are a contract (ADR 0047) |
| `describe [project] [--json]` | A project's details, apps (with what they open) and scripts; defaults to the project of the current folder |
| `commands [--json]` | The bus's command ids and titles |
| `open <project>` | Shows the project in the main window (`project:show:<id>`). Daybook calls `launcher open <id>` |
| `launch <project> <app>`, `run <project> <script>` | Opens an app; runs a script and prints its output (a failed script exits with 1 and says why; scripts wait up to 120 s) |
| `terminal`, `finder`, `config`, `db` `<project>` | A shell in the folder, the folder in Finder, the project file in an editor, the database in DataBee |
| `palette`, `settings`, `command <id>` | The palette, Settings, any bus command |
| `import <folder> [--yes]` | The prototype's data folder (its `projects.json`, `settings.json`): without `--yes` it lists what would change, with it imports as one undo step. Release builds only |

`<project>` is an id, a folder inside a project (`.`, `..`, `~/…` or a path with `/`, made absolute by the command
line) or a name: the exact name, then the folder's name, then a part only one project's name has. App and script
names match the same way, case-insensitively.

### The import

`automation/import.rs`: the prototype's projects join the list under their old ids, so links that name them (Daybook)
keep working; their name and details (`icon`, `color`, `tag`, `notes`, `starred`, `archived`) move into each
project's `.project-launcher.json`, where keys the file has already stay; missing folders and broken files are
skipped and listed. Its default editor, git client, terminal, project file editor, Docker app and commands fill the
settings, its tags join the tag list. Run it once without `--yes`, check the list, then with `--yes`.
