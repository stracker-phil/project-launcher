# Architecture

Launcher is a menu-bar app that keeps a registry of the owner's coding projects and launches their editor,
terminal, git client, URLs, scripts, dev servers and databases. Its moving parts are the Rust core (project
registry, config resolution, launchers, dev-environment status), the command bus, and three windows: the main
window, the Settings window and the floating command palette.

## Design rules

These rules hold everywhere in the code. Keep them when you change things.

1. **Rust owns the logic; React only renders** ([ADR 0001](adr/0001-rust-owns-the-logic.md)). Validation, ordering,
   filtering, grouping, dates and the CLI are Rust. The page asks, renders and forwards intent.
2. **IPC only in `api.ts` files** ([ADR 0002](adr/0002-ipc-goes-through-the-api-files.md)). Only `src/api/index.ts` (and
   `src/<module>/api.ts`) imports `@tauri-apps/*`. Everything else imports the `api` object, which is also
   what UI tests mock. `src/test/boundary.test.ts` enforces it.
3. **Errors are data** ([ADR 0003](adr/0003-errors-are-kinds-not-messages.md)). Every command returns `Result<T, AppError>`, serialized as
   `{ kind, message, field? }`. The UI branches on `kind` only.
4. **Types are generated** ([ADR 0004](adr/0004-ipc-types-are-generated-from-rust.md)). Every type crossing IPC derives `ts_rs::TS` and
   exports to `src/api/types.gen.ts` (camelCase). Never edit the generated file.
5. **Panics unwind** ([ADR 0005](adr/0005-panics-unwind.md)). A bug fails one command, not the app. Locks are taken with the
   poison-tolerant helpers in `lock.rs`.
6. **No color literal outside `src/styles/palettes/`** ([ADR 0025](adr/0025-every-color-is-a-palette-token.md)). A Vitest scan enforces it.
7. **The look is native and light** ([ADR 0024](adr/0024-native-look-with-the-main-window-following-the-prototype.md), [ux.md](ux.md)).
8. **One write path into the data** ([ADR 0007](adr/0007-one-write-path-for-app-data-and-project-files.md)). Every change goes through the data layer's one
   write function, which validates, saves and announces it as an event. No other code writes the data.
9. **Commands live in Rust** ([ADR 0008](adr/0008-commands-live-in-rust.md)). Menus, shortcuts, the palette, the CLI and links run
   commands of one bus by id.
10. **Small feature modules** ([ADR 0006](adr/0006-small-feature-modules.md)): by feature, pure cores, ports only where a
    swap is real, files under ~400 lines.
11. **Debug is a separate app on demo data** ([ADR 0022](adr/0022-the-debug-build-is-a-separate-app-on-demo-data.md)); nothing in the repo carries
    real data ([`demo-data.md`](../.claude/rules/demo-data.md)).
12. **Local first** ([ADR 0016](adr/0016-launcher-has-no-network-code.md)): no network at all, no secrets kept. Database passwords read
    from a dev server go to DataBee by environment variable and are never stored.

## Process model

- One process holds the window(s); the same binary is the CLI. The first process binds `launcher.sock` and
  holds `launcher.lock` in the config dir; a second one hands its request over and exits
  ([ADR 0009](adr/0009-the-cli-asks-the-running-app.md)).
- Windows are declared `visible: false` and shown by Rust, so `--hidden` never flashes them. Closing hides;
  only Quit ends the app.
- Launch requests are queued in Rust and pulled by the page (`take_launch_events`).
- `launcher://` links arrive through `tauri-plugin-deep-link` and run an allowlist of commands.
- Background work is supervised and restarted on panic ([ADR 0005](adr/0005-panics-unwind.md)): the dev-environment status
  poller (Docker, DDEV, wp-env; only while a window shows it), running scripts with captured output, and the
  project stats cache. Each change is announced as a Rust event that invalidates the page's queries
  ([ADR 0014](adr/0014-page-reads-go-through-tanstack-query.md)).
- The tray menu and the global shortcut (release builds only) run bus commands while every window is hidden
  ([ADR 0034](adr/0034-a-menu-bar-app-that-keeps-running.md), [ADR 0036](adr/0036-global-shortcuts-only-in-release-builds.md)).

## Data

JSON files only ([ADR 0012](adr/0012-json-files-only-no-database.md)), in the config dir through `JsonStore` (`store/json_store.rs` over
`store/json_file.rs`: atomic temp + rename, 0600 in a 0700 dir, defaults for a missing file, an unparsable file
moved aside as `<name>.<timestamp>.corrupt`, repair on load; [ADR 0013](adr/0013-json-stores-repair-themselves-on-load.md)):

| File | Holds |
|---|---|
| `projects.json` | `Registry` (`projects/registry.rs`): `{ "projects": [{ id, path, addedAt, lastOpenedAt }] }`, times in UTC ms; nothing else about a project ([ADR 0038](adr/0038-project-details-live-in-the-project-file.md)). Repaired per entry: no id or no absolute path → dropped; a second entry with the same id or path → dropped |
| `settings.json` | `Settings` (`store/settings.rs`): `appearance`, `tags` (the tag list: order, empty tags), `shortcuts` (overrides by command id in the page's `Mod+Shift+K` notation: a menu item's command such as `page:palette.open`, or a page command's id such as `settings.search`; `null` = no key), `defaultEditor`, `gitClient`, `terminalApp`, `configEditor`. `Settings::repaired` falls back per field |
| `<project>/.project-launcher.json` | Everything about the project (`projects/file.rs`, format in [project-file.md](project-file.md)): `name`, `meta` (icon, color, tag, notes, starred, archived, url, repoUrl, editor) and the launch config (apps, scripts, env, states, dev, database, locked; `projects/config/`). Read fresh before each change, written read-modify-write (two-space JSON, final newline, its own file mode); unknown keys stay in their place. A missing or broken file is never rewritten: the project shows `available: false` and `fileError` |

`data/` holds the `Data` the commands share (in `AppState`): both stores, the parsed project files cached by folder
(`Data::reload` empties the cache; file watching comes later) and the undo stack. `projects/view.rs` builds the
page's `Project` from an entry and its file (name from the file or the folder, icon `Folder` and colour `Blue`
when unset, notes as lines).

Every change goes through one writer, `Data::change` ([ADR 0007](adr/0007-one-write-path-for-app-data-and-project-files.md)): `data/edits/` (settings, tags, projects, config) checks the request against
the current data and turns it into an `Op` (`data/ops.rs`); applying an op returns its inverse, computed from
what it replaced, which goes on the bounded (100), non-persisted undo stack (`data/undo.rs`;
[ADR 0015](adr/0015-writes-are-operations-with-inverses-for-undo.md)). Ops: `SetAppearance`, `SetShortcut`, `ResetShortcut` (a reassigned key is a batch of two `SetShortcut`, a reset that takes its default from another command a `SetShortcut` and a `ResetShortcut`), `SetTags`, `AddProject`, `RemoveProject` (its inverse
re-inserts the entry at its place), `SetFileKey` (one key of a project file), `ReplaceFile` (a whole file; refused
when the file isn't what it was) and `Batch` (one undo step; a failure takes back what was applied). A tag
rename or delete is a batch of the tag list plus every project file carrying the tag; adding a folder without a
file is a batch that creates `{ "name": <folder> }` and adds the entry; an action edit sets `apps`, `scripts` or both. A
project with `"locked": true` refuses every detail edit except star, archive and unlock. An undo that can no longer apply is
dropped with its error. Recording a launch (`Data::touch_last_opened`) is not undoable.

Each write returns `Changed`, which `data::commands::announce` emits as `settings_changed` and
`projects_changed`; `src/queries/useEvents.ts` maps them to the query keys `["settings"]` (plus
`["menuShortcuts"]`) and `["projects"]`; both refresh `["projectConfig"]` (`useProjectConfig`), since the default editor and git
client resolve into it. A settings change, its undo and redo included, also rebuilds the menu bar
(`menu::refresh`) with the owner's keys.

The debug build writes the demo seed (`test_support/demo.rs`: made-up folders under `<data>/demo-projects/`, each
with a project file, plus the tag list) when its data folder has no `projects.json`; `launcher dev seed --yes`
writes it again and resets the data ([ADR 0022](adr/0022-the-debug-build-is-a-separate-app-on-demo-data.md)).

## Repository map

One line per module, added with the code.

| Path | Holds |
|---|---|
| `src-tauri/src/lib.rs` | Identity consts (`IDENTIFIER`, `APP_NAME`, `LINK_SCHEME`, tested against both Tauri configs) and `run`: CLI, handoff, start |
| `src-tauri/src/app.rs` | The Tauri builder: plugins, window events, setup, the run loop (only Quit ends it) |
| `src-tauri/src/cli/` | The `launcher` command line (clap) and how replies print (`reply.rs`) |
| `src-tauri/src/ipc/`, `instance.rs` | The Unix socket (0600, same user): wire format, client, server; the instance lock and handoff |
| `src-tauri/src/remote/` | What the running app does for each socket message |
| `src-tauri/src/launch.rs` | Launch events queued for the page (`take_launch_events`) |
| `src-tauri/src/window/` | Showing and hiding windows, the Dock icon; `settings.rs` opens the Settings window (at a page); `navigation.rs` keeps webviews on app pages |
| `src-tauri/src/bus/` | The command bus (`CommandBus`, `Command`), the app's commands (`commands.rs`), `run_command` and `spawn` (`app.rs`); ids in [commands.md](commands.md) |
| `src-tauri/src/menu/` | The menu bar: items per menu (`items.rs`), building it from the bus with the owner's keys (`build.rs`), `clicked`, `refresh`, `pause_menu_keys` (`MenuKeys`); `keys.rs` converts `Mod+Shift+K` ↔ `CmdOrCtrl+Shift+K` (`Combo`) and resolves a menu item's key (`effective_accelerator`); `palette.rs` lists the enabled items for the palette (`menu_commands`); `shortcuts.rs` lists every item's default and current key for Settings → Shortcuts (`menu_shortcuts`) and checks a new menu key (`checked`) |
| `src-tauri/src/tray.rs` | The menu-bar item; its items are bus commands |
| `src-tauri/src/folders.rs` | Revealing the log and data folders in Finder |
| `src-tauri/src/status.rs` | `app_status` for Settings → Status: version, build, folders, the `launcher` command link |
| `src-tauri/src/commands.rs`, `handlers.rs` | Core Tauri commands (`open_url`, `take_launch_events`, `log_frontend_error`) and the handler list |
| `src-tauri/src/error.rs`, `lock.rs`, `events.rs`, `state.rs` | `AppError`, poison-tolerant locks, event names, `AppState` |
| `src-tauri/src/store/` | Private folders, `JsonFile` and `JsonStore`, the `Settings` model and its repair |
| `src-tauri/src/projects/` | The registry (`registry.rs`), a project's `.project-launcher.json` (`file.rs`), the page's `Project` (`view.rs`), the launch config's model, resolver and editor (`config/`) |
| `src-tauri/src/data/` | `Data`: reads, the one writer (`change`), undo and redo (`mod.rs`, `undo.rs`); ops and inverses (`ops.rs`); the checked edits (`edits/`: `settings.rs`, `tags.rs`, `projects.rs`); Tauri commands and events (`commands.rs`); the Edit menu's Undo and Redo (`edit_menu.rs`) |
| `src-tauri/src/dev.rs`, `test_support/` | Debug-only hooks (frame, background, eval, demo seed); test fixtures and the demo seed (`demo.rs`) |
| `src/api/` | The page's only Tauri boundary (`api`, `toAppError`) and the generated types |
| `src/commands/` | Page commands: the registry, the owner's overrides and key matching (`registry.ts`), the keydown hook (`useShortcuts.ts`), the overrides from `settings.json` (`useShortcutOverrides.ts`), each window's commands (`mainCommands.ts`, `settingsCommands.ts`), `useMainCommands`, `useSettingsCommands`, the Edit menu's Undo and Redo per window (`useEditRequests.ts`) |
| `src/windows/settings/` | The Settings window's page (`index.html`, `main.tsx`, `SettingsApp.tsx`), a second Vite entry |
| `src/registry/settingsPages.ts` | Settings pages by id: title, icon, keywords, searchable rows, component; resolving and searching them; `useSettingsPagesVersion` re-renders the window when a page registers again |
| `src/settings/` | The Settings window's layout (`SettingsWindow.tsx`), groups, rows and `Segmented` (`controls.tsx`), the core pages (`pages.ts`: Appearance, Shortcuts, Status, About); `shortcuts/` is Settings → Shortcuts (`ShortcutsPage.tsx`, `ShortcutRecorder.tsx`, the rows and conflicts in `rows.ts`) |
| `src/palette/` | The command palette (`CommandPalette.tsx`, rows in `PaletteRows.tsx`): its sources (`sources.ts`: menu items, page commands, Settings pages, `usePaletteMenu` menus from `menus.ts`), what it shows (`sections.ts`), ranking (`rank.ts`), prefixes (`prefixes.ts`), recent searches in memory (`recent.ts`) |
| `src/queries/` | TanStack Query: `createQueryClient` (one per window), the read hooks (`useAppStatus`, `useSettings`, `useMenuShortcuts`, `useProjects`, `useProjectConfig`) and `useEvents` (Rust events → query keys) |
| `src/lib/` | Hooks and helpers shared by windows (`useLaunchEvents`, `theme.ts` for the scheme, `motion.ts` for presence and glides) |
| `src/state/attempt.ts` | `attempt`: a request whose failure shows as an error toast |
| `src/state/store.ts` | The page's zustand store: the window layout (sidebar, inspector per layout), toasts, the confirmation sheet, the palette's open state, the open menu; `window.launcher` in debug builds |
| `src/state/menuTypes.ts` | The menu's types (`MenuItem`, `MenuEntry`, `MenuAt`, `OpenMenu`) and `isItem`, shared by the store, `src/components/menu/` and the palette |
| `src/shell/` | The main window's panes: sidebar, project list, detail, inspector slot |
| `src/components/` | Overlays every window has: `TooltipHost`, `Toasts`, `Modal`, `ConfirmSheet`; `SegmentedControl`; `menu/` is the in-page menu (`openMenu`, `openContextMenu`, `MenuHost`, `MenuPanel`, `placement.ts`; ADR 0030) |
| `src/styles/` | Stylesheets in cascade order (`index.ts`); `palettes/` holds every color; `contrast.ts` for the palette tests |
| `src/test/` | `createApiMock()` and the module-boundary guard |

## Webview hardening

- CSP in `src-tauri/tauri.conf.json` (`csp` and `devCsp`): `script-src 'self'`, `style-src-elem 'self'`, no
  frames, objects, base URI or form actions, `connect-src` only to IPC ([ADR 0017](adr/0017-hardened-runtime-strict-csp-and-no-navigation.md)).
- `window/navigation.rs` keeps every webview on the app's own pages; web links go through `open_url`.
- The bundle is signed with the hardened runtime (`scripts/sign-app.sh`).

## Platform

macOS on Apple Silicon only; see [adapting.md](adapting.md) for every macOS- or name-specific place.
