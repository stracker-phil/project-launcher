# Launcher

A macOS menu-bar app to list, search and manage your coding projects and launch what each one needs: its
editor, terminal, git client, URLs, Claude Code, scripts, local dev server (Docker, DDEV, wp-env) and its
database in [DataBee](../databee).

Launcher is a personal tool, published as source. It runs on macOS on Apple Silicon; it has no network code,
no telemetry and keeps its data in `~/Library/Application Support/com.philippstracker.launcher/`.

> Work in progress: the app is being built step by step. Features land as they are described here.

## What it does

- Keeps a list of project folders with a name, icon, colour, tag, notes, star and archive flag.
- Searches them from the main window, the command palette (⌘K) or the menu-bar icon; ⌃⌘L shows the window
  from anywhere.
- Launches each project's apps and scripts from its `.project-launcher.json` in the project root, a file you
  can edit by hand or in the app.

## Install

```sh
bun install
bun run release patch   # builds, signs and installs ~/Applications/Launcher.app and ~/.local/bin/launcher
```

See [AGENTS.md](AGENTS.md) for prerequisites and [docs/](docs/README.md) for how it is built.

## Command line

The app binary is also the CLI; it asks the running app.

```sh
launcher status           # is it running?
launcher show             # show the main window
launcher list --json      # the projects
launcher open <project>   # show a project
launcher quit
```

## License

MIT, see [LICENSE](LICENSE).
