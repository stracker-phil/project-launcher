# 0034. Launcher is a menu-bar app; closing a window keeps it running

## Status

Accepted (2026-11-11). Inherited from the family catalog (app-blueprint: `closing-keeps-running`, Choice: menu-bar app); reference implementation: TimeTracker ADR 0007.

## Context

A launcher is summoned many times a day and should be there from login. The prototype was an accessory app:
a tray icon with recent projects, the Dock icon only while a window was open, started hidden.

## Decision

Launcher has a tray icon (open, palette, recent projects, the current project's actions, Settings, Quit).
The Dock icon shows only while the main or Settings window is open, never for the palette. Closing a window
(red button, ⌘W) hides it; only Quit ends the app; `RunEvent::ExitRequested` without a code is ignored.
Windows are declared `visible: false` and shown by Rust, so `--hidden` never flashes them. Launch at login
is a LaunchAgent with `--hidden` through `tauri-plugin-autostart`. Launch requests are queued in Rust and
pulled by the page (`take_launch_events`).

- Rejected: a Dock app (Taskboard): no quick access from the menu bar, Dock icon always present.

## Consequences

- Commands from the tray and CLI run while every window is hidden.
- Dock visibility follows window visibility; switching it needs care with activation policy timing.
