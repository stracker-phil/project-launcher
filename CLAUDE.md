# Launcher

Launcher is a menu-bar app that lists, searches and manages the owner's coding projects and launches what each
one needs: its editor, terminal, git client, URLs, Claude Code, scripts, its local dev server (Docker, DDEV,
wp-env) and its database in DataBee. Everything about a project (name, icon, colour, tag, notes,
starred, archived and its launch config) lives in the hand-editable `.project-launcher.json` in its root;
Launcher's data folder keeps only its settings, the tag list and the list of known project files with their
last-opened times ([ADR 0038](docs/adr/0038-project-details-live-in-the-project-file.md)). It does not edit code, sync anything or reach the network. It replaces a
prototype built on Glaze (Electron); that prototype is the mockup and the source of the logic to migrate. The
step plan and migration notes live in `docs/plan/` (gitignored); build only what the current step asks for.

Launcher is part of the owner's family of macOS apps and inherits its architecture, process and look from the
family catalog (the user-level `app-blueprint` skill). The sibling apps (leave out this app's own entry):

- **Taskboard** (`../taskboard`): the newest decisions, scripts and process
- **Daybook** (`../daybook`): the command bus, extra windows, the menu bar
- **TimeTracker** (`../timetracker`): SQLite with sqlx, undo, seeds, MCP
- **DataBee** (`../databee`): plugins and ports

The prototype lives outside the family; its path is in `docs/plan/steps.md` and its `ARCHITECTURE.md` maps
it. Read it for behavior to migrate; never write to it, and never copy the owner's projects from it.

Read them; never write to them. Launcher differs only where its job differs; each deviation has an ADR.

## Rules

- Every session or step ends in a commit, so it can be reverted. Commit each finished part of a step as soon as
  it is green, not only at the end.
- Never push. Only commit locally.
- Write unit tests for every change and run them (through the `ci` agent) before every commit; tests and
  lint must be green.
- Tests pin behavior, not internals: public interfaces only, no log assertions, no call counts.
- Refactor continuously: when a file grows large (over ~400 lines of production code) or complex, simplify it
  and split it into focused modules in the same step, without asking.
- Keep modules small, isolated and replaceable ([ADR 0006](docs/adr/0006-small-feature-modules.md)): by feature, pure cores, a
  trait only where a second implementation exists today. No speculative abstractions.
- Demo data only: code, docs, tests, commits and screenshots never contain the owner's real data
  (`.claude/rules/demo-data.md`). The debug build runs on its own data folder with the demo seed.
- UI changes follow `docs/ux.md` and `.claude/rules/stable-controls.md`.
- The implementation plan lives in `docs/plan/` (gitignored). Work one step per session; set its status from
  PENDING to DONE when it is committed.
- Changes to the family catalog go through the owner (`app-blueprint`, mode `record`); never edit it from here.

## Docs
Technical docs live in `docs/` (index: `docs/README.md`). Read `docs/architecture.md` and the page of the
layer you change before changing it; `docs/ux.md` is the design language; `docs/adapting.md` lists everything
macOS- or name-specific. Decisions are in `docs/adr/` (never edited once accepted; a new record supersedes).
Update the affected docs page in the same commit when behavior, commands, files or extension points change.
Document by file and symbol, never by line number.

## Stack
- Rust backend in `src-tauri/` (rustup stable, edition 2024; Homebrew rustup is keg-only:
  `/opt/homebrew/opt/rustup/bin`)
- Tauri 2 + React 19 + TypeScript in `src/`, bun as package manager, zustand, lucide-react, Vitest
- macOS, Apple Silicon only

## Architecture
- Rust owns all logic; React only renders ([ADR 0001](docs/adr/0001-rust-owns-the-logic.md)). Only `api.ts` files call
  `invoke`/`listen` ([ADR 0002](docs/adr/0002-ipc-goes-through-the-api-files.md)).
- Errors cross the boundary as `{ kind, message }` ([ADR 0003](docs/adr/0003-errors-are-kinds-not-messages.md)). IPC types are generated with
  ts-rs ([ADR 0004](docs/adr/0004-ipc-types-are-generated-from-rust.md)).
- Commands live in a Rust bus; menus, shortcuts, palette, CLI and links run them by id
  ([ADR 0008](docs/adr/0008-commands-live-in-rust.md)).
- The CLI is the same binary and asks the running app over a Unix socket ([ADR 0009](docs/adr/0009-the-cli-asks-the-running-app.md)).
- No color literal outside `src/styles/palettes/` ([ADR 0025](docs/adr/0025-every-color-is-a-palette-token.md)); the look is native
  ([ADR 0024](docs/adr/0024-native-look-with-the-main-window-following-the-prototype.md), `docs/ux.md`).
- Debug builds: `Launcher (Debug)`, `com.philippstracker.launcher.debug`, own data and socket, demo seed, dev
  hooks; never installed ([ADR 0022](docs/adr/0022-the-debug-build-is-a-separate-app-on-demo-data.md)).
- Data is JSON files only (`projects.json`, `settings.json` through `JsonStore`; [ADR 0012](docs/adr/0012-json-files-only-no-database.md)); project details live in
  each project's `.project-launcher.json` ([ADR 0038](docs/adr/0038-project-details-live-in-the-project-file.md)); every
  write, including to a project's `.project-launcher.json`, goes through one writer ([ADR 0007](docs/adr/0007-one-write-path-for-app-data-and-project-files.md))
  as an op with an inverse, so ⌘Z undoes it ([ADR 0015](docs/adr/0015-writes-are-operations-with-inverses-for-undo.md)).
- Page reads go through TanStack Query; Rust events invalidate query keys ([ADR 0014](docs/adr/0014-page-reads-go-through-tanstack-query.md)).
- Appearance is Automatic / Light / Dark with one palette each ([ADR 0026](docs/adr/0026-automatic-light-or-dark-with-one-palette-each.md)).
- No network: no HTTP client in the crate ([ADR 0016](docs/adr/0016-launcher-has-no-network-code.md)).
- A menu-bar app: tray icon, Dock icon only while a window is open, launch at login hidden
  ([ADR 0034](docs/adr/0034-a-menu-bar-app-that-keeps-running.md)); ⌃⌘L shows it, release builds only ([ADR 0036](docs/adr/0036-global-shortcuts-only-in-release-builds.md)).
- The command palette is a floating panel on the native window layer ([ADR 0037](docs/adr/0037-the-palette-is-a-floating-panel-on-a-native-window-layer.md)).
- DataBee is opened through its CLI; Daybook calls `launcher list --json` / `launcher open <id>`
  ([ADR 0011](docs/adr/0011-siblings-talk-through-their-clis.md)).
- Repository map: `docs/architecture.md`.

## Commands
Test, lint and format-check commands run through the `ci` agent, never in the main context.
- `bun run test`: Vitest and Rust unit tests
- `bun run lint`: IPC types drift check, tsc, ESLint and clippy on production code (warnings are errors)
- `bun run release major|minor|patch`: bumps the version, builds the signed release `.app` and `.dmg`, installs
  it (`scripts/install.sh`), commits "Release X.Y.Z", tags it and cleans the build folder (`scripts/release.sh`,
  `scripts/clean-build.sh`, [ADR 0020](docs/adr/0020-every-release-build-gets-a-new-version.md)); major only with the
  owner's approval ([ADR 0039](docs/adr/0039-a-major-version-needs-the-owners-approval.md))
- `bun run build:app` + `scripts/dev/launch.sh`: debug build, restarted next to the terminal without stealing focus
- `scripts/stop-app.sh <debug CLI path>`: whoever launches the debug app for a check quits it when done
- `scripts/dev/js.sh` / `scripts/dev/screenshot.sh`: drive (store: `window.launcher`) and capture the running
  debug window in the background; `scripts/dev/e2e.sh` checks that nothing jumps
- Signing prefers an Apple Development certificate (Xcode, free Apple ID; [ADR 0019](docs/adr/0019-builds-are-signed-with-an-apple-development-certificate.md)); without one,
  `scripts/create-signing-cert.sh` (once per Mac) makes the local fallback that keeps privacy grants

## Workflow
Finish every step with the `ship-step` skill. Rules in `.claude/rules/`; `testing.md` says which agents in
`.claude/agents/` write and run tests.
