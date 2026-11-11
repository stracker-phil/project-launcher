# 0026. Automatic, Light or Dark, with one palette each

## Status

Accepted (2026-11-11). Inherited from the family catalog (app-blueprint: `appearance`, Choice: Automatic / Light / Dark); reference implementation: Taskboard ADR 0028.

## Context

The prototype offered a theme setting with System, Light and Dark and one designed look. Its sidebar stayed
dark in both modes. Launcher has no accent-driven tints; the project colors carry the color.

## Decision

`settings.appearance` is `system`, `light` or `dark`. `light.css` and `dark.css` each cover the whole
window; `src/lib/theme.ts` sets `data-scheme` before the first render and follows macOS on Automatic. No
accent choice. The prototype's always-dark sidebar, if kept, is expressed as sidebar tokens inside each
palette (`--sidebar-*`), decided in the look step; no separate scheme.

- Rejected: a scheme per macOS mode plus an accent (Daybook, TimeTracker): more palettes and settings than
  one designed look needs. Editor schemes (DataBee): Launcher shows no code.

## Consequences

- Two palette files to keep complete; the token tests catch gaps.
- A dark sidebar in light mode is a token decision, not a second theme.
