# 0024. Native look, with the main window following the prototype

## Status

Accepted (2026-11-11). Inherited from the family catalog (app-blueprint: `native-look`); reference implementation: Taskboard ADR 0010 and Taskboard ADR 0017.

## Context

Launcher sits beside Finder, the terminal and the editor and should look like them. The owner already approved
a working prototype: a dark sidebar with library, tags and archive; a searchable project list with Recent and
All sections; a detail view edited in place; an inspector with project stats.

## Decision

Launcher follows the family language (`docs/ux.md`): system font at macOS text styles, tabular figures, SF Mono
only for machine text (paths, commands, branch names), no bundled fonts; neutral greys, inset groups without
borders, an overlay title bar where empty chrome drags the window, AppKit control metrics, the accent only on
interactive things, the sheet's default button the only filled one. The main window's layout follows the
owner-approved prototype in pixels (sidebar, project list, detail, inspector, including the project colour
tinting the selected row), mapped onto the family tokens; Settings, menus, the palette, tooltips and behaviour
follow the family language, not the prototype's component library.

- Rejected: cards with borders, filled web buttons and chips; copying the prototype's Glaze components as they
  are (another design system to keep in step); redesigning the main window from scratch (the owner already
  uses and approved it).

## Consequences

- The migration ports layout and behaviour, not components; each view is checked against the prototype's
  screenshots.
- Where the prototype and the family language disagree outside the main window, the family wins.
