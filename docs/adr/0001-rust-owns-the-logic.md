# 0001. Every rule lives in Rust; the page asks, renders and forwards intent

## Status

Accepted (2026-11-11). Inherited from the family catalog (app-blueprint: `rust-owns-the-logic`); reference
implementation: Taskboard ADR 0001.

## Context

The Glaze prototype split its logic: the backend resolved project configs and ran tools, while the page
filtered, sorted, grouped ("Recent" within 72 hours, then "All"), ranked palette hits and built the menu model.
Some rules existed twice (the settings whitelist in the handler, the defaults in the page) and only the
backend parts had tests.

## Decision

Every rule lives in Rust: the project registry, resolving `.project-launcher.json` (presets, `${dir}`
substitution, hidden states), validation and settings repair, filtering by library and tag, sorting and the
Recent section, search and palette ranking, shortcut conflicts, git and manifest facts, and the text the app
shows. The React page asks for data, renders it and sends intent (an id, a patch, a command id).

- Rejected: keeping the prototype's split; rules end up duplicated or untested.
- Rejected: moving view state (selection, open popovers) into Rust; that stays in the page.

## Consequences

- Each rule has one copy, unit-tested without a DOM, and the CLI gets the same answers as the window.
- Migrating a prototype feature means porting its TypeScript rule to Rust, not copying it into `src/`.
