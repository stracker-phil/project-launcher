# 0010. Times are stored as UTC milliseconds and formatted by the platform

## Status

Accepted (2026-11-11). Inherited from the family catalog (app-blueprint: `rust-formats-time-and-dates`);
reference implementation: TimeTracker ADR 0011.

## Context

Launcher shows when a project was last opened ("opened 5 min ago"), the Recent section (opened within the last
72 hours), last-commit dates and the inspector's "updated 2 min ago". The prototype stored ISO strings and
formatted them in the page by hand.

## Decision

Timestamps (`lastOpenedAt`, `createdAt`, stats `computedAt`) are stored as UTC milliseconds (`i64`). Date math
such as the Recent cut-off is Rust. Absolute dates are formatted in Rust with the macOS region
(`NSDateFormatter`); relative times that tick every minute may use the page's built-in `Intl.RelativeTimeFormat`.
No date library and no hand-written formatting in the page.

- Rejected: moment, date-fns or dayjs; hand-rolled formatting; ISO strings parsed in the page.

## Consequences

- Dates follow the owner's region settings without app code.
- Old ISO strings from the prototype's `projects.json` are converted once on import.
