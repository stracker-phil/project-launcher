# 0028. Motion uses shared tokens and controls stay where they are

## Status

Accepted (2026-11-11). Inherited from the family catalog (app-blueprint: `motion-and-stable-controls`); reference implementation: Taskboard `docs/ux.md` and
`src/lib/motion.ts` (no Taskboard ADR).

## Context

A launcher is used in quick bursts, often by keyboard and muscle memory. The prototype revealed controls on
hover and reordered the Recent section as projects were opened; anything that jumps under the cursor breaks
that.

## Decision

Motion goes through the tokens in `src/styles/motion.css` (`--motion-fast` 120ms, `--motion-base` 150ms,
`--motion-slow` 200ms, `--ease-out`, `--ease-in-out`, a global `prefers-reduced-motion` rule) and the helpers
in `src/lib/motion.ts` (`useGlide`, `useListGlide`, `useHeightGlide`, `usePresence`, `useFlash`). A click
never changes what is under the cursor; lists keep their order while open (a project opened from the list
moves into Recent on the next visit, not under the cursor); toggles keep their size; optional buttons keep
their slot; `.motion-off` shows a new state at once where a glide would mislead. `scripts/dev/e2e.sh`
checks for drift.

- Rejected: ad-hoc durations per component; reordering lists live; hiding buttons by removing them.

## Consequences

- The same click does the same thing again; e2e fails on 1px drift.
- Hover-revealed controls keep their slot and only fade.
