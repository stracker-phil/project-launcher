# Launcher design language

The family design language (from the `app-blueprint` catalog) plus Launcher's own window layout. The full
token lists and CSS rules are in Taskboard's `docs/ux.md` and `src/styles/`; copy from there and record
Launcher's values here as they land.

## Launcher's windows

Launcher follows its prototype mockup for the main window ([ADR 0024](adr/0024-native-look-with-the-main-window-following-the-prototype.md)); Settings and behavior follow
the family language below.

### Main window
- **Sidebar** (dark, as in the mockup; expressed as sidebar tokens in each palette, [ADR 0026](adr/0026-automatic-light-or-dark-with-one-palette-each.md)): the
  current library's title and project count at the top; a Library group (All Projects, Starred), a Tags group
  (inline new and rename) with Untagged, and Archived last. Hover "+" adds a project or a tag. Toggle:
  ⌃⌘S.
- **Project list**: a full-width search field on top; sections "Recent" (opened in the last 72 hours) and
  "All", both alphabetical, so rows never re-sort while the list is open. The selected row takes the
  project's colour. Right-click opens the project menu.
- **Detail**: icon and colour, name, tag and notes, all edited in place (no dialogs); an Open grid (editor,
  terminal, git client, URLs, Claude Code, custom apps); Scripts with Run and the last run's output; Info
  (website, repository, location, git, editor, config file); Dev Environment (Docker, DDEV or wp-env, and
  the database that opens in DataBee). Locked projects show everything read-only.
- **Inspector** (right, toggle ⌃⌘I): overview, languages, git activity, recently edited files and package
  scripts; slow parts load on their own.
- No status bar ([ADR 0035](adr/0035-no-status-bar.md)).
- Pane widths (from the prototype): sidebar 240px, list 300px, detail at least 380px, inspector 280px; each
  pane's 52px top strip drags the window. Hiding the sidebar also hides the list and leaves room for the
  traffic lights in the detail's strip. The inspector remembers open or closed for each layout: closed
  beside the sidebar, open without it until changed (`src/state/store.ts`).

### Command palette
A floating panel, not part of the main window ([ADR 0037](adr/0037-the-palette-is-a-floating-panel-on-a-native-window-layer.md)): centred over the main window,
or on the screen while the main window is hidden; opens with ⌘K, from the tray and while the app is in the
background. Prefixes: `>` commands, `:` projects, `#` library, `/` the current project's actions.

### Tray menu
Open Launcher (⌃⌘L), Command Palette, Recent Projects ▸ (⌥1 … ⌥0), Settings…, Quit, then the current
project's apps and scripts.

## Principles
- Calm, light and native: it looks like the Mac apps beside it (`native-look`). The window opens at
  once; nothing animates for show.
- Fully keyboard-usable: every action is a command with a menu item, a palette entry and a rebindable
  shortcut (`keyboard-first`).
- One word for each thing, in every label, menu item, palette title, toast and the docs; accessible names
  follow the visible labels.
- Demo data in every picture.
- Stable controls (below).
- All configuration lives in Settings.

## Stable controls
- A click never changes what is under the cursor; a toggle toggles back; only closing an overlay or
  navigating away may replace what was clicked.
- Lists don't re-sort while in use; a removed or ticked row keeps its place (grayed) until the page is left
  or the pointer leaves.
- Controls keep their size: state shows by color and outline, never an added icon or text; buttons that
  only sometimes apply keep their slot, invisible.
- Nothing jumps, things move: fade in, glide (`useGlide`, `useListGlide`), animate height
  (`useHeightGlide`). A window resize may move things at once.
- Folding into "…" follows the window width only.
- Before finishing a UI change: click every new or changed control twice and run `scripts/dev/e2e.sh`.

## Text and controls never wrap
Buttons, tabs, chips, column headers and table cells stay on one line; long text truncates with an ellipsis
(which shows its full text in a tooltip). When space runs out the flexible element shrinks, never a button.
Only descriptions, bodies and empty-state text wrap.

## Type and metrics
- System font (SF Pro), SF Mono only for machine text, no bundled fonts, tabular figures for numbers.
- Baseline scale 13 (controls, body) / 11 (meta) / 15 (section) / 17+ (titles); an app with a mockup may
  use a 14px body. Text size Small/Medium/Large with ⌘= / ⌘- / ⌘0 where reading matters.
- 8px grid, 20px margins. Radii: 6 (chips, row hover), 7 (pills, small buttons, menu rows), 8 (sidebar rows,
  toolbar buttons), 10–12 (groups, panels, popovers), 16 (palette), 50% (dots, avatars).
- Unified toolbar ~50–52px with 22–30px controls; traffic lights native inside the sidebar's top strip
  (`trafficLightPosition`); every empty strip drags the window.

## Tokens
Palettes define the same token set (`color-palettes`); groups: Surfaces (`--window-bg`, `--panel-bg`,
`--fill-subtle`, `--fill-hover`, `--segmented-bg`, `--scrim`), Text (`--text`, `--text-secondary`,
`--text-tertiary`, `--text-muted`, `--text-faint`, `--danger-text`, `--text-on-accent`, `--link`), Lines
(`--border`, `--border-strong`, `--hairline`), Accent (`--accent`, `--selection-*`, `--button-primary-*`,
`--focus-ring`), Messages (`--banner-{info,warn,error}-*`), Shadows (`--shadow-{panel,popover,palette,…}`),
Hover (`--hover-bg`, `--row-hover-bg`), Settings (`--settings-*`), Tooltip (`--tooltip-bg`). The text
contrast checks: 4.5:1 body, 3:1 meta and large text.

Launcher's palettes (`src/styles/palettes/`) also define Controls (`--control-bg`, `--control-pressed`,
`--control-shadow`, `--field-bg`, `--field-line`), Sidebar tokens that are the same in both schemes
(`--sidebar-bg` #212121, `--sidebar-text`, `--sidebar-text-tertiary` for headings, `--sidebar-hover-bg`,
`--sidebar-selection-bg`, `--sidebar-border`) and the seven project colors by name (`--project-blue`,
`-green`, `-magenta`, `-orange`, `-purple`, `-red`, `-yellow`). One accent (`--accent`), no accent choice.
Body text is 13px. An app using the scheme-per-mode option
(`appearance`) also has sidebar tokens and opacity levels (`--o2` 68%, `--o3` 52%; see Daybook `docs/ux.md`
→ Surfaces, Opacity levels).

## Motion
`--motion-fast` 120ms (hover, press), `--motion-base` 150ms (fades, chevrons, toggles, popover exit),
`--motion-slow` 200ms (expand/collapse, popover entry), `--ease-out`, `--ease-in-out`; no literal durations
elsewhere; `prefers-reduced-motion` makes movement instant and fades short. Popovers rise in and fade out
(`usePresence`, inert while leaving). A view switch is instant. Expand/collapse via `Disclosure`: height
animates, rows fade, what's below glides.

## Scrollbars
None are styled; every scroll box has `.scroller` (`isolation: isolate`) and starts below its title row;
menus and modals portal to `<body>` (`overlay-scrollbars`).

## Menus (`menus-in-the-page`)
12px radius, `--border`, `--shadow-popover`, 6px padding; rows 30px, 7px radius, 13px text; optional 18px
icon, hint or key (muted), check or ›; hover `--hover-bg`, disabled `--text-faint`, destructive
`--danger-text`; caps caption; hairline between groups. Opens at the mouse or under its button, flips, then
shifts inside the window. Closes on Esc, outside press, scroll, blur, resize or a choice; focus returns; a
second press on the button closes. Keys: ↑↓ wrapping, Home/End, ↩/Space, letters jump, →/← submenus;
command keys are off while a menu has focus. Long lists get a search field. ARIA `menu`/`menuitem`/
`menuitemradio`/`menuitemcheckbox`.

## Tooltips (`tooltip-host`)
One host per window; `title` becomes `data-tooltip` (icon-only controls keep `aria-label`); 450ms, instant
within 600ms, on focus; below or above, ≤420px, line breaks kept; ends with the current binding
("Only mine (⌘⇧M)").

## Overlays inside a window
Toasts bottom right, self-dismissing (success 2.5s, info 4s, error 8s, with an action 10s), repeats count up
(×3); errors read "Something went wrong" with Details. Sheets (`Modal`) over `--scrim` take Esc first,
close on a scrim click, trap Tab and return focus. Anything that loses the owner's setup asks first in a
`ConfirmSheet` that names what goes and what stays; Cancel and Esc keep it. The content never blanks: it
keeps the previous view until the next is loaded; a skeleton only at launch, after 250ms. Empty states say
why in few words and offer one button per narrowing to clear it. Reverting a remote write says
"Change back to …", not "Undo".

## Command palette (`palette-runs-the-menus`)
⌘K; panel 640px, 16px radius, `--shadow-palette`, 110px from the top, over `--scrim`; input row 56px with a
17px field; prefixes shown as footer buttons; recent searches; rows show the current key and a check for
checked options; ↑↓ wrap, ↩ runs, Esc closes.

## Settings (`settings-window`)
Own window in System Settings layout: 190px page list on `--settings-chrome` with drag strip and search;
page title in the strip; groups (13px semibold heading, inset `--settings-fill-quiet`, 12px radius) of rows
(label and description left, control right, hairlines). Controls: TextField, NumberField, DraftField with
Rust's field error in red below, Segmented, switches; destructive buttons ask in an alertdialog sheet. With
another window in front, switches, primary buttons and the list turn gray (`window-blurred`). Generic pages:
Appearance, Shortcuts, Status, About (version, Show Data Folder, Open Log Folder), Privacy & Data.

## Keys
App actions are Rust commands; page keys live in the page registry, reached from menus as `page:<id>`. Menus
in Mac order; View holds window appearance only; Help holds Open Log Folder and Show Data Folder. Bare keys
never fire in text fields (only ⌘/⌃ combinations and Esc); plain letters stay off menu items; one binding per
command.

## Palette checks
`colors.test.ts` keeps literals in palettes and token sets equal; `contrast.test.ts` enforces the contrast
targets; `scripts/dev/screenshot-looks.sh <dir>` captures the window in both palettes for review.
