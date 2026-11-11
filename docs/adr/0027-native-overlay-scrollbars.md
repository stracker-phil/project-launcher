# 0027. Scrollbars are the native overlay bars

## Status

Accepted (2026-11-11). Inherited from the family catalog (app-blueprint: `overlay-scrollbars`, Recommended); reference implementation: DataBee ADR 0032.

## Context

The project list, the detail pane and the palette results all scroll. A single `::-webkit-scrollbar` rule
makes WebKit draw a permanent bar, which no other Mac app does.

## Decision

No `::-webkit-scrollbar` rule anywhere and no `scrollbar-gutter`. Every user-scrollable box has the
`.scroller` class (`overflow: auto; isolation: isolate`) from `src/styles/base.css` and starts below its
title row; menus and modals portal to `<body>`. A scroller where an overlay bar would hide content the owner
needs may reserve space, recorded in its own ADR.

- Rejected: styled bars; JS-faded bars; a stable gutter (an empty strip beside overlay bars).

## Consequences

- Bars appear only while scrolling, as on the Mac.
- New scroll containers must use `.scroller`; review checks for stray scrollbar CSS.
