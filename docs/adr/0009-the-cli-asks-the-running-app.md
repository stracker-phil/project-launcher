# 0009. The `launcher` binary is also the CLI and asks the running app

## Status

Accepted (2026-11-11). Inherited from the family catalog (app-blueprint: `cli-asks-the-running-app`);
reference implementation: Taskboard ADR 0008.

## Context

The prototype's `plaunch` was a shell script that encoded its arguments into a URL, ran `open -g`, and polled
a reply file in the app's data folder. It needed reinstalling per build and could not stream errors reliably.
Daybook calls `plaunch list --json` and `plaunch open <id>` today.

## Decision

The app binary is the CLI. The first process binds `launcher.sock` (0600, peer-uid check) and holds
`launcher.lock` in the config dir. A second process parses argv with clap, sends one JSON line, prints the
reply (`--json` for machines) and exits. `show`, `focus` and a bare launch start the app; other subcommands
exit 2 with "Launcher isn't running". Commands: `status [--json]`, `quit`, `--hidden`, `list [--json]`,
`open <id>`, `describe`, `launch`, `run`, `command <id>`; debug builds accept `eval`. `list --json` and
`open <id>` keep the shapes Daybook parses today, so Daybook only swaps the tool name.

- Rejected: keeping the URL-scheme script with reply files; a CLI that reads `projects.json` itself;
  `tauri-plugin-single-instance` (argv lands in logs).

## Consequences

- One implementation of every action, with undo and events; no second writer.
- `scripts/install.sh` links `~/.local/bin/launcher`; the old `plaunch` is retired once Daybook switches.
