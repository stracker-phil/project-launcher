# Adapting Launcher

Everything that is specific to the name, to macOS or to the owner's setup, for renaming, porting or forking.
`CLAUDE.md` points to `.claude/rules/`; the `ship-step` skill runs after every step; `.claude/agents/` hold
the subagents for tests, suites and reviews. Every new name-, path- or platform-specific place gets a row here.

## Identity

| What | Release | Debug | Where |
|---|---|---|---|
| Product name | `Launcher` | `Launcher (Debug)` | `tauri.conf.json` `productName` and window title, `tauri.debug.conf.json`, `lib.rs` `APP_NAME`, `index.html`, the `app:show`/`app:quit` titles (`bus/commands.rs`) and the Quit item (`menu/items.rs`) |
| Bundle identifier | `com.philippstracker.launcher` | `com.philippstracker.launcher.debug` | both configs and `lib.rs` `IDENTIFIER` (tested); names the config dir and logs |
| Binary and CLI | `launcher` | the same, called by path | Cargo package name (also the self dev-dependency), `cli/` |
| Installed links | `~/Applications/Launcher.app`, `~/.local/bin/launcher` | never installed | `scripts/install.sh` |
| URL scheme | `launcher://` | `launcher-debug://` | both configs and `lib.rs` `LINK_SCHEME` (tested) |
| Socket and lock | `launcher.sock`, `launcher.lock` in the config dir | its own | `lib.rs`, `instance.rs`, `scripts/dev/js.sh` |
| Config dir | `~/Library/Application Support/<identifier>/` | its own | `lib.rs` `config_dir()` |
| Logs | `~/Library/Logs/<identifier>/` | its own | Tauri `app_log_dir` |
| Signing | `LAUNCHER_SIGN_IDENTITY`, else Apple Development, else `Launcher Local Signing` | same | `scripts/sign-app.sh` |
| Env vars | `LAUNCHER_SIGN_IDENTITY` | adds `LAUNCHER_WINDOW_FRAME`, `LAUNCHER_BACKGROUND`, `LAUNCHER_DEV_TERMINAL` | `dev.rs`, `scripts/dev/` |
| Dev store | — | `window.launcher` (`VITE_DEV_HOOKS=1`) | `src/main.tsx` |
| Data | the owner's | the demo seed, its folders in `<config dir>/demo-projects/` | `src-tauri/src/test_support/demo.rs`, `dev.rs` |
| Project file | `.project-launcher.json` in each project folder | same | `projects/file.rs` `FILE_NAME` |
| Author, license | Philipp Stracker <info@philippstracker.com>, MIT | | `Cargo.toml`, `LICENSE` |

## Network hosts

None: the app has no network code and no listener besides its Unix socket (a `lib.rs` test fails on an HTTP
client or server among the direct dependencies; [ADR 0016](adr/0016-launcher-has-no-network-code.md)). A new
host or listener needs an ADR and a row here. Links open in the browser: https anywhere, http only to a local
name (`links.rs`, [ADR 0040](adr/0040-links-open-over-https-or-http-to-a-local-name.md)).

## Platform-specific code

- macOS on Apple Silicon only: the config path, Unix sockets, the overlay title bar, the Dock model,
  codesigning (`scripts/sign-app.sh`, `scripts/create-signing-cert.sh`).
- Dev scripts use zsh, Swift (`CGWindowList`) and `screencapture`.
- The floating palette panel (`NSPanel`, window level, Space behavior) on the native window layer
  ([ADR 0037](adr/0037-the-palette-is-a-floating-panel-on-a-native-window-layer.md)).
- AppleScript (`osascript`) to drive other apps, with the Apple Events entitlement in
  `src-tauri/entitlements.plist` ([ADR 0017](adr/0017-hardened-runtime-strict-csp-and-no-navigation.md)).
- The tray icon and the Dock icon shown only while a window is open; launch at login as a LaunchAgent
  (`tauri-plugin-autostart`; [ADR 0034](adr/0034-a-menu-bar-app-that-keeps-running.md)).
- Launching apps with `open -a`, terminal sessions through `.command` files, scripts in a login `zsh -l`.

## Common changes

| Goal | Start here |
|---|---|
| A new command or shortcut | `src-tauri/src/bus/`, the menu table in `src-tauri/src/menu/`, `docs/commands.md`, `docs/ux.md` → Keys |
| A new setting | the `Settings` model and `Settings::repaired` in `src-tauri/src/store/settings.rs`, a page in `src/registry/settingsPages.ts` |
| A new Tauri command | the feature's `commands.rs`, `handlers.rs`, `src/api/index.ts`, `src/test/apiMock.ts` |
| A new CLI subcommand | `src-tauri/src/cli/`, `src-tauri/src/remote/` |
| A new color | a token in every palette under `src/styles/palettes/` |
| A new built-in app preset (editor, terminal, git, …) | the launcher feature in `src-tauri/src/`, `docs/adapting.md` |
| A new key in `.project-launcher.json` | the config model and resolver (`projects/`), an op in `data/ops.rs` when Launcher edits it; keep unknown keys on write |
| A new dev-server tool | the dev-environment feature, its fake in `test_support/` |

## Conventions to keep

- Rust owns the logic; the page renders. IPC only in `api.ts` files; types generated with ts-rs.
- Small feature modules; split files over ~400 lines.
- No color literal outside the palettes; the native look of `ux.md`.
- Tests through public interfaces only; test code written and run by the agents.
- Debug is a separate app on demo data; nothing in the repo carries real data.
- Docs by file and symbol, never by line number; an ADR for every decision, never edited once accepted.
