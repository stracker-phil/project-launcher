# 0045. Settings groups are Taskboard's inset cards; no headline repeats

## Status

Accepted (2026-11-12). Owner's decision. Supersedes the "groups without a fill" part of
[0042](0042-settings-follow-timetracker-with-an-accent-choice.md); its accent-filled page list, accent choice,
intros and 48px rows stay. Reference: Taskboard's `src/styles/settings.css` and Settings → Shortcuts.

## Context

The family's three Settings windows had drifted apart: Taskboard showed groups on a filled card, TimeTracker and
Launcher without one, and each Shortcuts page grouped, labeled and spaced its rows differently. The owner named
Taskboard's Settings the reference for groups and the Shortcuts page. Taskboard also repeats one word as page
title, group heading and row label (Appearance › Appearance › Appearance), which the owner rules out.

## Decision

- Every Settings group is an inset card: heading 16px in, then rows on `--settings-fill-quiet` with a 12px radius,
  16px side padding and hairlines between rows; 24px between groups.
- No headline repeats the one above it: a group heading never restates the page title, a row label never
  restates its group heading or page title, and a row description never restates its group heading. A page
  with a single group may leave the group without a heading.
- Settings → Shortcuts groups its rows by where the key works: "In every window" (the menu's items, in menu
  order, each described by its menu, e.g. "File menu"), "In the main window", "In the Settings window".
- A row counts as changed, and shows its reset button, only while its key differs from the default. Recording
  the default key resets the row instead of storing an override.
- Rejected: TimeTracker's unfilled groups (the owner prefers Taskboard's cards); Taskboard's gray current page
  (ADR 0042's accent stays); a scope description on every shortcut row as in Taskboard (it would repeat the
  group heading).

## Consequences

- The palettes gain `--settings-fill-quiet`.
- Appearance's mode row is "Mode" and its group has no heading; About's group has no heading.
