# Launcher technical documentation

These docs are for developers and coding agents working on the code. For what the app does and how to
use it, see the [project README](../README.md).

| Page | Covers |
|---|---|
| [architecture.md](architecture.md) | Design rules, process model, data, repository map, webview hardening |
| [project-file.md](project-file.md) | `.project-launcher.json`: keys, built-ins, placeholders, resolving and editing |
| [commands.md](commands.md) | The command bus, every command id, page commands and the menus |
| [ux.md](ux.md) | Design language: window, tokens, type, metrics, menus, tooltips, motion, Settings, keys |
| [testing.md](testing.md) | Unit and integration tests, the test agents, the demo seed, scripts that drive the app, signing |
| [adapting.md](adapting.md) | Identity (names, ids, paths), network hosts, platform-specific code, conventions to keep |
| [adr/](adr/README.md) | Architecture decision records: why the design is the way it is |

Pages for each layer and feature are added with the code they describe, in the same commit.

Suggested reading order: architecture, then the layer you are changing, then adapting.

The rules in [`.claude/rules/`](../.claude/rules/) are binding for changes:
- [`testing.md`](../.claude/rules/testing.md)
- [`commits.md`](../.claude/rules/commits.md)
- [`demo-data.md`](../.claude/rules/demo-data.md)
- [`stable-controls.md`](../.claude/rules/stable-controls.md)
- [`rust-test-conventions.md`](../.claude/rules/rust-test-conventions.md)
- [`ts-test-conventions.md`](../.claude/rules/ts-test-conventions.md)

## Sibling apps

Launcher is one app of the owner's family and shares their stack, process and look (family catalog: the
user-level `app-blueprint` skill). When this repo has no answer yet, look there (read-only):

| App | Path | Take from it |
|---|---|---|
| Taskboard | `../taskboard` | the newest decisions: scripts, ship-step, demo seed, menus, palette, motion, tooltips, Settings window, status bar, CSP and navigation guard, network gateway, rusqlite |
| Daybook | `../daybook` | the Rust command bus (`docs/commands.md`), extra windows on a native layer, menu-bar app, global shortcuts, local HTTP API, JSON stores with repair |
| TimeTracker | `../timetracker` | SQLite through sqlx, undo as ops, the seeds and showcase, performance tests, socket `watch`, the MCP server |
| DataBee | `../databee` | optional features as inline plugins, ports with fakes (`Database`, `SecretStore`), the `Secret` type |
