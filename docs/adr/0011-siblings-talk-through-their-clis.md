# 0011. Launcher talks to DataBee and Daybook only through their CLIs

## Status

Accepted (2026-11-11). Inherited from the family catalog (app-blueprint: `sibling-apps-talk-through-the-cli`);
reference implementation: Daybook ADR 0022.

## Context

Two siblings are involved. Outgoing: a project with a MySQL/MariaDB database (DDEV, wp-env or a `database`
URL in `.project-launcher.json`) opens in DataBee. Incoming: Daybook lists Launcher's projects and shows one.

## Decision

Outgoing, Launcher starts the `databee` CLI with host, port, user and database as arguments and the password
in an environment variable named by `--password-env`, never in argv (`src-tauri/src/databee/`). It never reads
DataBee's files. Incoming, Daybook calls `launcher list --json` and `launcher open <id>` (ADR 0009); Launcher
keeps those shapes stable. The debug build talks only to `databee-debug` or a stand-in script, never to the
release DataBee, and sends no mutating command to a release sibling; `LAUNCHER_REAL_DATABEE=1` switches to the
real tool for checking that code path.

- Rejected: writing DataBee's connection files; opening DataBee through AppleScript; a shared database.

## Consequences

- Each app keeps its own validation; the CLI is the contract and its JSON shape needs care when it changes.
- A missing `databee` command shows a clear error ("isn't on your PATH") instead of failing silently.
