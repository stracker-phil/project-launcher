# 0032. Every action has a menu item, a palette entry and a rebindable shortcut

## Status

Accepted (2026-11-11). Inherited from the family catalog (app-blueprint: `keyboard-first`); reference implementation: DataBee ADR 0036.

## Context

The prototype already made menu accelerators the single shortcut path and let each project action carry its
own shortcut. Launcher keeps that, on the family's command bus.

## Decision

Every action is a command with a menu item, a palette entry and one rebindable binding; Settings → Shortcuts
flags conflicts, including with per-project action shortcuts. Menus in Mac order: Launcher, File, Edit, View,
Library, Project, Window, Help; Help has Open Log Folder and Show Data Folder. Bare keys never fire in text
fields; plain-letter keys stay off menu items. `app:settings:<page>` deep-opens Settings.

- Rejected: page-only key handlers; fixed shortcuts; shortcuts without a menu item.

## Consequences

- Everything is reachable without the mouse and discoverable in the menu bar.
- A new command must declare its menu placement and default key.
