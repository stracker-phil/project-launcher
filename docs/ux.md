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
- **Detail**: icon and colour, name, the tag pill and the git branch pill (a long name cut short, whole in its
  tooltip), and notes, all edited in place (no
  dialogs); then, in this order: Dev Environment (a status dot, Docker and the DDEV or wp-env server with
  "Running · Docker Desktop" / "Running · <host>", a globe button for the site, Restart then Start/Stop, and the
  database that opens in DataBee); an Open grid (each action's icon on a tile in its colour, what it opens, its
  key as keycaps); Scripts (icon, label, the command in mono, a Run button) with the last run's output; Info
  (website and repository with open buttons and a pencil that edits them, dev mode, location, editor). Section
  titles are small and muted; content sits in outlined cards (hairline border, 12px radius). The toolbar's
  lock, star and inspector buttons are round and filled. Locked projects show everything read-only. Paths in
  the home folder show from `~`.
- **Project list rows** carry a green dot while the project's dev server runs, beside the star.
- **Inspector** (right, toggle ⌃⌘I): "Details" with a round Refresh button; Overview (Type, Requires, Packages,
  Files, Lines, sizes, Last edited), Languages as a table, Git (branch, changes, remote, commits, started, latest
  tag, last commit, twelve weeks of green activity bars), Recently Edited and Package Scripts, each section
  folding with its chevron; slow parts load on their own.
- No status bar ([ADR 0035](adr/0035-no-status-bar.md)).
- Pane widths (from the prototype): sidebar 240px, list 300px, detail at least 380px, inspector 280px; each
  pane's 52px top strip drags the window. Hiding the sidebar also hides the list and leaves room for the
  traffic lights in the detail's strip. The inspector remembers open or closed for each layout: closed
  beside the sidebar, open without it until changed (`src/state/store.ts`).

### Command palette
A floating panel, not part of the main window ([ADR 0037](adr/0037-the-palette-is-a-floating-panel-on-a-native-window-layer.md)): centred over the main window,
or on the screen while the main window is hidden; opens with ⌘K, from the tray and while the app is in the
background. Prefixes: `>` commands, `:` projects, `#` library, `/` the current project's actions, its dev
environment's steps that apply now (Start Docker, Stop DDEV, Open Site…) and its Project menu items as they apply
(Unstar, Unarchive, Unlock by state; Move to Tag: <tag>), as the prototype's menu listed them
(`docs/commands.md` → The palette).

### Tray menu
Open Launcher, Command Palette…, Recent Projects ▸ (⌥1 … ⌥0), then the current project's name (gray) with its
apps and scripts (their keys as hints) and Reveal in Finder, then Settings… and Quit.

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
`-green`, `-magenta`, `-orange`, `-purple`, `-red`, `-yellow`). The accent is the owner's choice
([ADR 0042](adr/0042-settings-follow-timetracker-with-an-accent-choice.md)): each palette defines the nine
hues (`--accent-hue-orange` … `-graphite`, each 4.5:1 under `--text-on-accent`) and the lightness range the
accent keeps as text (`--accent-text-min-l`, `--accent-text-max-l`); `src/styles/accent.css` sets `--accent`
from `data-accent` (System: `AccentColor`) and derives `--accent-text` and `--selection-text`. Settings also
has `--settings-focus-ring`, `--settings-knob`, `--settings-knob-shadow`, `--settings-control-pressed` and
`--settings-blurred-fill`.
Body text is 13px. An app using the scheme-per-mode option
(`appearance`) also has sidebar tokens and opacity levels (`--o2` 68%, `--o3` 52%; see Daybook `docs/ux.md`
→ Surfaces, Opacity levels).

## Motion
`--motion-fast` 120ms (hover, press), `--motion-base` 150ms (fades, chevrons, toggles, popover exit),
`--motion-slow` 200ms (expand/collapse, popover entry), `--motion-highlight` 1600ms (a found Settings row's wash), `--ease-out`, `--ease-in-out`; no literal durations
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

Launcher's values (`src/components/menu/`, `src/styles/menu.css`): 200–400px wide, keeps its opening width
while a search narrows it; 6px from the window's edges, 4px below its button; a submenu lines up with its row;
rises in and fades out (`usePresence`, inert while leaving).

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

Launcher's values (`src/styles/palette.css`, `src/palette/`): for now an overlay in the main window (it moves
into its own panel, [ADR 0037](adr/0037-the-palette-is-a-floating-panel-on-a-native-window-layer.md)); scrim fades in, panel rises in. Rows 40px, 9px
radius, 14px text, the active row `--hover-bg`; a muted key chip at the right; caps section headings (11px)
with a hairline between sections. Empty, it shows the recent searches (this session only, five at most) and
every command under its menu; with a query, one list ranked: label starts with → a word starts with → every
word → letters in order, ties in menu order. Only prefixes with something to find get a footer button (`>`
so far). ⌘K or a click on the scrim closes it as well; the focus returns to where it was.

## Settings (`settings-window`)
Own window in System Settings layout: a page list under group headings on `--settings-chrome` with drag
strip and search; page title in the strip, a one-line intro below it; groups (13px semibold heading) of rows
(label and description left, control right, hairlines) on an inset card. No headline repeats the one above it
(page title, group heading, row label, row description; [ADR 0045](adr/0045-settings-groups-are-taskboards-inset-cards.md)). Controls: TextField, NumberField, DraftField with
Rust's field error in red below, Segmented, switches; destructive buttons ask in an alertdialog sheet. With
another window in front, switches, primary buttons and the list turn gray (`window-blurred`). Generic pages:
Appearance, Shortcuts, Status, About (version, Show Data Folder, Open Log Folder), Privacy & Data.

Launcher's values (`src/styles/settings.css`, `src-tauri/src/window/settings.rs`; TimeTracker's page list,
[ADR 0042](adr/0042-settings-follow-timetracker-with-an-accent-choice.md), and Taskboard's inset groups,
[ADR 0045](adr/0045-settings-groups-are-taskboards-inset-cards.md)): window 880×720, min 760×560, not
maximizable, traffic lights at 20,22; 36px strips (drag region in the list, page title 15px bold in the pane);
page list 176px, pages under 11px semibold group headings (App: General, Appearance, Shortcuts · System: Status,
Privacy & Data, About; `GROUP_ORDER` in `src/registry/settingsPages.ts`), rows 28px with 6px radius, the current
page filled with `--accent` and `--text-on-accent` (icon and label inherit); search field 28px on
`--settings-fill`; page body 8/20/24px padding, 24px between groups; a one-line intro
(`--settings-text-secondary`) under each page title; groups as inset cards (`--settings-fill-quiet`, 12px radius, 16px side padding), 13px semibold heading
16px in, an optional description below it, a page with one group may leave it without a heading, hairlines (`--settings-hairline`) between rows; rows at least 48px (14px
vertical padding), the label column flexible (a real `<label>` for its control), the control at its own size;
a row can stack its control below the label or show dimmed (`Row stacked`, `disabled`); values
`--settings-text-secondary`, selectable and cut off with an ellipsis; push buttons and fields 28px with 8px
radius. Fields (`src/settings/fields.tsx`: `TextField`; number and draft fields join when a page needs one) save
when left or on ↵; a value Rust refuses for its field (`InvalidField`) shows its message in red below it,
any other failure as a toast. Switches are checkboxes (`role="switch"`) 32×18 with a `--settings-knob`; pop-up
buttons (`PopUpButton`) open the window's menu (`MenuHost`). With another window in front, the current page,
checked switches and primary buttons turn `--settings-blurred-fill` (`window-blurred` on <html>,
`src/lib/windowFocus.ts`). The search ([ADR 0046](adr/0046-settings-search-finds-every-row.md)) turns the page
list into one list of every matching page and row, ranked as in the palette, a row hit showing its page as listed with the row's title on a second line
("No Results" when none); pages match by title, group and keywords, rows by title and keywords. Each page module
exports its rows (`*_ROWS`, `SettingsRow` in `src/registry/settingsPages.ts`): every `Row` has an `anchor`, its
entry's `id`, and its label is the entry's `title`; a test holds them together. A hit opens its page scrolled to
the row, which an accent wash highlights once (`--motion-highlight`); picking it again does so again. ↵ opens the
first hit, ⌘F focuses the field, ⎋ clears it, then ⎋ and ⌘W hide the window. Segmented: a `--settings-fill` track with 2px padding, 24px segments, the pressed one a
`--settings-control` tile with `--settings-control-shadow`. Pages so far:

- General: the default editor, git client, terminal and project file editor (text fields; the terminal can't
  be empty and no value may hold a line break, refused in red below the field), the Docker app (a pop-up
  button: Docker Desktop, OrbStack, Colima, Custom; Custom adds its start and stop commands as fields), and
  Open at login (a switch; dimmed in the debug app, which never installs a login item).
- Appearance: Mode, Automatic / Light / Dark as three tiles, each a miniature window in its palette (Automatic split
  light and dark), the chosen one ringed in the accent; Accent color as swatches, System (the macOS accent
  color) then orange, gold, red, magenta, purple, indigo, blue, teal and graphite. Every window switches at once.
- Shortcuts: a "Search commands" field narrows the rows by title, group or menu ("No matching commands."
  when none is left); rows grouped by where the key works: "In every window" (every menu item, in menu order,
  described by its menu, e.g. "File menu"), then "In the main window" and "In the Settings window" (the page
  keys no menu item holds); each row is found by the Settings search too. A row shows its
  key in a 7rem recorder field: a click records the next key (the menu's keys are off meanwhile, so ⌘W or ⌘Q
  reach it), ⌫ leaves the command without one, ⎋ or a click elsewhere cancels; an unbound key reads "None" in
  muted italics. Left of it, the reset button (only while the key differs from the default; recording the default key resets the row) and the × remove button
  (only while there is a key) keep their slots when hidden. A key another command has asks first ("⌘K is used
  by “Command Palette”", Reassign), then moves it as one undo step; so does a reset to a default another
  command has now; menu keys clash with every key, page keys only within their window. A menu key without ⌘
  or ⌃ is refused with a toast.
- Status: version, build, data and log folder, the `launcher` command link.
- Privacy & Data: what Launcher keeps in its data folder (settings, shortcuts and the tag list; the known
  project files with their last-opened times; the main window's frame and sidebar choice), that everything else about a project lives in its own
  `.project-launcher.json`, Show Data Folder, and that Launcher has no network code (links open in the browser).
- About: Version, Data (Show Data Folder), Logs (Open Log Folder).

## Keys
App actions are Rust commands; page keys live in the page registry, reached from menus as `page:<id>`. Menus
in Mac order; View holds window appearance only; Help holds Open Log Folder and Show Data Folder. Bare keys
never fire in text fields (only ⌘/⌃ combinations and Esc); plain letters stay off menu items; one binding per
command. The owner's keys (Settings → Shortcuts, `settings.json` `shortcuts`) override the defaults: Rust
rebuilds the menu bar with them and the page registry uses them for page keys. The menu bar and the tray are
rebuilt only when their keys or rows change, never for another setting: a rebuild makes macOS redraw every
menu title.

## Palette checks
`colors.test.ts` keeps literals in palettes and token sets equal; `contrast.test.ts` enforces the contrast
targets; `scripts/dev/screenshot-looks.sh <dir>` captures the window in both palettes for review.
