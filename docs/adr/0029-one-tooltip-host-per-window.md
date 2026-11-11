# 0029. One tooltip host per window replaces native tooltips

## Status

Accepted (2026-11-11). Inherited from the family catalog (app-blueprint: `tooltip-host`); reference implementation: Daybook ADR 0039.

## Context

Launcher's icon tiles, action buttons and truncated project paths need explanations and shortcut hints.
Native `title` tooltips are slow and can hang over the floating palette.

## Decision

One `TooltipHost` per window (main, Settings, palette) takes over every `title`: after 450ms, instantly
within 600ms of the last one, on keyboard focus, and for ellipsized text; below the target (above if no
room), up to 420px wide, 8px inside the window; hidden by press, key, wheel or leaving. Tooltips end with the
command's current binding from `useShortcuts`; no tooltip spells a key itself.

- Rejected: native `title`; a `<Tooltip>` wrapper per control.

## Consequences

- Controls only set `title`; shortcut hints follow rebinding automatically.
- Each new window mounts its own host.
