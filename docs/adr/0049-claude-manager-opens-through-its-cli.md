# 0049. Claude Manager opens a project through its CLI; the Project menu follows the shown project

## Status

Accepted (2026-11-18). Applies [ADR 0011](0011-siblings-talk-through-their-clis.md) and Claude Manager's ADR 0011
(its Launcher contract) to Claude Manager; [ADR 0041](0041-debug-builds-record-databee-like-every-launch.md)'s
debug recording covers it. The menu follows the family catalog's `menu-bar-updated-in-place`.

## Context

The prototype offered "Manage Claude" for folders with a `.claude` directory while `claude-manager` was on the
login shell's PATH, passing the profile as `CLAUDE_CONFIG_DIR` in the env. Claude Manager is now a sibling app
whose CLI asks the running app, so an env var no longer reaches it; it takes the profile as `--profile`, and
without one keeps the profile it has open. The menu bar's Project menu left every item enabled whatever project
was shown, while the right-click menu disabled what didn't apply.

## Decision

- **Manage Claude** (`page:project.claude`, ⇧⌘M) runs `claude-manager open <folder> --profile <profile>` through
  the `System` port (`claude_manager.rs`), always with the profile: `CLAUDE_CONFIG_DIR` from the project's env,
  else `~/.claude`. Claude Manager validates the folders and reports a missing one.
- **Installed** means an executable `claude-manager` in `~/.local/bin` (where its release installs it) or
  Homebrew's folders, checked by file, not by a login shell, so every read can ask.
- **Available** when the folder has `.claude` and the CLI is installed (`Project.canManageClaude`). Unavailable,
  the item is disabled in the menu bar and the right-click menu, and left out of the palette; the detail's Info
  row "Claude" shows the profile with its Manage button disabled.
- **The Project menu follows the shown project:** Rust disables each Project item that doesn't apply, in place,
  by the rule the palette uses (`project_items.rs` `applies`), and every item while no project is shown.

- Rejected: hiding the item when unavailable (the menu would change length); passing the profile in the env
  (it doesn't reach a running app); a `command -v` in a login shell on every read (slow, needs a cache).

## Consequences

- A `claude-manager` installed elsewhere isn't found.
- The debug build records the command instead of opening Claude Manager, unless `LAUNCHER_REAL_LAUNCH=1`.
