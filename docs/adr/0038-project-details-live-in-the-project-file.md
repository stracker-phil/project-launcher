# 0038. Every project detail lives in the project's own file

## Status

Accepted (2026-11-11). Supersedes the parts of [0012](0012-json-files-only-no-database.md) that keep launcher-only
metadata in `projects.json`; [0007](0007-one-write-path-for-app-data-and-project-files.md),
[0013](0013-json-stores-repair-themselves-on-load.md) and [0015](0015-writes-are-operations-with-inverses-for-undo.md)
apply unchanged to the new split.

## Context

The prototype first kept a project's name, icon, colour, tag, notes, starred and archived flags in its
`.project-launcher.json`, then moved them into its own `projects.json`. That split a project's description in two:
the launch config in the project, the rest in one Mac's app data. A project moved to another Mac, opened from a
second checkout or restored from a backup lost its details, and the CLI and siblings had two places to read.

## Decision

A project's `.project-launcher.json` holds everything about the project: its name, `meta` (icon, colour, tag,
notes, starred, archived, URLs, editor) and its launch config (apps, scripts, env, states, dev services,
database, locked), in the prototype's original keys. Launcher's own data holds only what is about the app:

- `settings.json`: the settings, and the tag list (order and empty tags), which spans projects.
- `projects.json`: the known project files (id, folder path, added time) and each one's last-opened time,
  which changes on every launch and would otherwise rewrite the project file each time.

Every edit of a project detail is a read-modify-write of its file through the one writer, keeping unknown keys,
as an op with an inverse. A project whose file is missing or unreadable stays listed, marked unavailable, with
its folder name; it is never recreated silently.

- Rejected: launcher-only metadata in `projects.json` (the prototype's last state): details don't travel with
  the project.
- Rejected: last opened in the project file: a write, and a diff in a committed file, on every launch.
- Rejected: starred and archived in app data: they are the owner's view of the project, and the owner wants
  every project detail in its file.

## Consequences

- Adding a project with no file creates a minimal `.project-launcher.json` (its name) in the folder.
- A tag rename or delete rewrites the file of every project with that tag, as one undoable op.
- Loading the list reads one small file per project; Launcher caches them and re-reads a file on change.
- The one-time import from the prototype writes its `projects.json` details back into each project's file.
