# 0042. Settings follow TimeTracker, with an accent choice

## Status

Accepted (2026-11-12). Owner's decision. Supersedes the "no accent choice" part of
[0026](0026-automatic-light-or-dark-with-one-palette-each.md); its Automatic / Light / Dark with one palette
each stays. Reference: TimeTracker ADRs 0022 and 0035.

## Context

Launcher's Settings window took Taskboard's look: the current page in a gray fill, inset groups on a quiet fill,
one fixed blue accent and no way to change it. The owner wants the family's Settings to look like TimeTracker's,
above all its appearance page and its accent handling, and wants to pick Launcher's accent.

## Decision

- The Settings window takes TimeTracker's look: the current page filled with the accent and on-accent text,
  pages under group headings (App, System) with a one-line intro under each title, groups without a fill and
  with hairlines between rows, the title in bold, TimeTracker's sidebar width. Rows keep Launcher's 48px.
- `settings.accent` is `system` (the macOS accent color, WebKit's `AccentColor`) or one of nine named hues
  (orange, gold, red, magenta, purple, indigo, blue, teal, graphite). It changes through the one writer as an op
  with an inverse, like the appearance, and every window follows. `src/lib/theme.ts` sets `data-accent` on
  <html>; `src/styles/accent.css` points `--accent` at the palette's `--accent-hue-<name>`, and everything
  derived from it (selection, focus rings, primary buttons, switches, the current Settings page) follows in
  both palettes. Each hue reads 4.5:1 under `--text-on-accent`.
- Settings → Appearance shows the three modes as preview tiles (Automatic split light and dark) and the accent
  as swatches.
- Project colors stay their own named hues; the accent never changes them.
- Rejected:
  - Taskboard's gray selection and no accent choice: the owner wants TimeTracker's look and a choice;
  - TimeTracker's scheme per macOS mode: Launcher keeps one palette per mode and an app override (ADR 0026);
  - TimeTracker's 36px rows: Launcher's 48px rows stay, as in System Settings.

## Consequences

- The palettes gain the nine hues, the text lightness range for the accent and the Settings control tokens;
  the contrast tests cover the hues.
- With System, the accent can be any color macOS offers, also one that reads below 4.5:1 under white, as in
  macOS itself.
