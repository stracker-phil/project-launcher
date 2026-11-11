# The project file

Every project keeps everything about itself in `.project-launcher.json` in its root
([ADR 0038](adr/0038-project-details-live-in-the-project-file.md)). The format is the prototype's, so existing files work unchanged.
The owner edits it by hand too: Launcher reads it fresh before each change, touches only the keys it changes and
keeps every other key, in its place.

Code: `src-tauri/src/projects/file.rs` (read, write, the keys Launcher changes), `projects/view.rs` (name and
`meta` for the list), `projects/config/` (the launch config: `model.rs`, `vars.rs`, `resolve.rs`, `builtins.rs`, `edit.rs`),
`data/edits/config.rs` and `data/edits/projects.rs` (the edits, as undoable ops).

## Keys

```json
{
  "name": "Acme Website",
  "meta": {
    "icon": "Globe", "color": "Blue", "tag": "Client", "notes": ["Relaunch in March"],
    "starred": true, "archived": false,
    "url": "https://acme.example.com", "repoUrl": "https://git.example.com/acme/website", "editor": "Zed"
  },
  "locked": false,
  "dev": { "ddev": true },
  "database": "mysql://user:password@127.0.0.1:3306/acme",
  "env": { "APP_ENV": "local", "CLAUDE_CONFIG_DIR": "~/.claude-work" },
  "apps": [
    "editor",
    { "preset": "terminal", "label": "Shell", "shortcut": "cmd+shift+t" },
    { "label": "API Docs", "url": "${url}/docs", "icon": "Book" },
    { "label": "Tail Logs", "command": "tail -f ${dir}/logs/app.log", "hiddenStates": ["paused"] }
  ],
  "scripts": [{ "label": "Build", "command": "npm run build", "icon": "Hammer", "shortcut": "cmd+shift+b" }],
  "stateProviders": { "queue": "cat ${dir}/queue-state" },
  "states": {
    "queue": {
      "source": "queue", "label": "Queue",
      "values": { "paused": { "value": "paused", "label": "Paused", "color": "Orange" } }
    }
  }
}
```

| Key | Meaning |
|---|---|
| `name`, `meta` | The project's name and details (the list and the detail pane); `notes` is a list of lines or one string |
| `locked` | `true` makes the details read-only in Launcher: name, icon, colour, tag, notes, actions, built-ins and the dev mode refuse changes; starring, archiving and unlocking still work |
| `dev` | What runs while the owner works on the project: `{ "docker": true }`, `{ "ddev": true }` or `{ "wpEnv": true }` (DDEV and wp-env imply Docker; DDEV wins over wp-env over Docker) |
| `database` | Absent: found in the DDEV or wp-env setup; `false`: hidden; a `mysql://` URL: this database. The URL never reaches the page |
| `env` | Variables for terminals, scripts and state providers; numbers and booleans count as text |
| `apps` | Built-ins by name (`"editor"`) or with overrides (`{ "preset": "editor", … }`), and actions of their own: `label` plus `app` (and `args`), `url`, or `command` (a terminal; empty opens a shell in the folder) |
| `scripts` | `label` and `command`, run in a login shell with their output captured |
| `stateProviders`, `states` | Commands whose output shows as states on the project; `source` names a provider, or `provider:.path` a value in its JSON output. A state's `values` map outputs to labels; an action's `hiddenStates` lists the value keys that hide it |

Every action can have `icon`, `color` and `shortcut` (`cmd+shift+k`, the prototype's notation; the page shows and
records keys as `Mod+Shift+K` and Launcher writes them back in the file's notation).

## Built-ins

| Name | Label | Opens | Unavailable when |
|---|---|---|---|
| `editor` | Edit Code | `meta.editor`, else the default editor (Settings) | neither is set |
| `terminal` | Open Terminal | a shell in the folder | never |
| `git` | Open Git Client | the git client (Settings) | the folder has no `.git`, or no git client is set |
| `browser` | Open in Browser | `meta.url` | no URL |
| `repoBrowser` | Open Repository | `meta.repoUrl` | no repository URL |
| `claude` | Claude Code | `claude` in a terminal | never |

An unavailable built-in is left out of the project's actions; the action editor lists it with the reason.

## Placeholders

Commands, URLs, arguments, env values, state sources and provider commands fill in `${dir}` (the project
folder), `${url}` (`meta.url`; left as written without one) and `~` where it starts a word and is followed by
`/` or the end (`~/x`, `cd ~/x`, `"~/x"`; not `a~/x` or `~user`).

## Resolving

`projects::config::resolve` turns a file into the page's `ProjectConfig`: `apps` and `scripts` as `Action`s (label,
icon, colour, shortcut, how it runs, target, `hiddenStates`) with the placeholders filled, entries that don't fit
skipped; all six built-ins with `enabled` and `unavailableReason`; `env`, `stateProviders` and `states` in file
order; `dev`, `database` (`detect`, `hidden` or `url`), `locked`, `isGitRepo` and `claude` (when the folder has a
`.claude` directory, with `CLAUDE_CONFIG_DIR`). Read with `project_config(id)`; a missing or broken file is an
error, as the project shows unavailable.

Each action carries `edit`, what the action editor starts from: `source` (list and index in the file), `draft`
(the fields as written, no placeholders filled; for a built-in only its overrides), `preset` and, for a built-in,
`defaults` (what each field falls back to).

## Editing

Starring a project removes `archived` and archiving removes `starred`, in the same undo step. `set_project_meta`
also sets `url` and `repoUrl` (the Info section's Website and Repository).

Commands: `save_action(id, source | null, draft, preset | null)`, `remove_action`, `move_action(id, source, to)`,
`set_builtin_enabled`, `set_project_locked`, `set_dev_mode`, plus `set_project_name` and `set_project_meta` for the
details. `projects::config::edit::Lists` changes copies of `apps` and `scripts` and returns the lists that changed;
each changed list becomes a `SetFileKey` op, so one edit is one undo step even when an action moves between lists.

- A draft writes `label`, the target fields of how it runs (others are dropped), `icon`, `color` and `shortcut`;
  empty fields are removed, other keys of the entry stay in place.
- A built-in stays its bare name until something overrides it, then becomes `{ "preset": …, … }`.
- Changing a script into anything else (or back) moves it to the end of the other list.
- An index that no longer exists means the file changed in the meantime: the edit is refused.
- Enabling a built-in appends its name unless it is listed; disabling removes every entry of it.
