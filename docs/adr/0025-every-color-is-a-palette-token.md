# 0025. Every color is a palette token, checked by tests

## Status

Accepted (2026-11-11). Inherited from the family catalog (app-blueprint: `color-palettes`); reference implementation: Taskboard ADR 0007.

## Context

The Glaze prototype took its colors from the Glaze design system and a hand-copied `.force-dark` block that
had to be regenerated whenever the SDK theme changed. Launcher also tints rows and icon tiles with each
project's own color, so stray literals would spread quickly.

## Decision

Every color is a CSS custom property defined in `src/styles/palettes/` (`light.css`, `dark.css`); both
palettes define the same tokens. Everything else uses `var()`, `color-mix()` or relative OKLCH. Project
colors are named hues (the Apple system colors) stored by name, never free hex. Vitest scans fail on a stray
literal or a missing token (`src/styles/colors.test.ts`) and on low contrast (`src/styles/contrast.test.ts`:
4.5:1 body text, 3:1 meta and large text).

- Rejected: colors from a component library's theme (the prototype's Glaze tokens); hex values in
  components; free-form project colors.

## Consequences

- A new look is a palette file, not a refactor; a missing dark value fails a test.
- Project tints must be expressed as mixes of a named hue with palette tokens.
