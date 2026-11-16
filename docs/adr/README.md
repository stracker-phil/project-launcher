# Architecture decision records

Why Launcher is built the way it is. One record per decision, in Nygard format, at most one page:
Context, Decision, Consequences.

- An accepted record is never edited. A changed decision gets a new record that supersedes the old one; the
  old one's Status only gains "Superseded by NNNN". A dated amendment under Status may fix facts that changed.
- Status is dated. The Decision names the alternatives it rejected.
- Records taken from the family catalog (the `app-blueprint` skill) say so under Status, with the slug and the
  sibling record that implements it; they state the decision as it holds for Launcher.

| ADR | Decision | Status |
|---|---|---|
| [0001](0001-rust-owns-the-logic.md) | Every rule lives in Rust; the page asks, renders and forwards intent | Accepted (2026-11-11) |
| [0002](0002-ipc-goes-through-the-api-files.md) | Only the API files talk to Tauri; the rest of the page imports `api` | Accepted (2026-11-11) |
| [0003](0003-errors-are-kinds-not-messages.md) | Errors cross the boundary as a stable kind, never as parsed messages | Accepted (2026-11-11) |
| [0004](0004-ipc-types-are-generated-from-rust.md) | IPC types are generated from Rust with ts-rs | Accepted (2026-11-11) |
| [0005](0005-panics-unwind.md) | Panics unwind, so a bug fails one command and not the app | Accepted (2026-11-11) |
| [0006](0006-small-feature-modules.md) | Code is organised in small feature modules with pure cores | Accepted (2026-11-11) |
| [0007](0007-one-write-path-for-app-data-and-project-files.md) | App data and each project's config file each have exactly one write path | Accepted (2026-11-11) |
| [0008](0008-commands-live-in-rust.md) | App actions are commands of one Rust bus | Accepted (2026-11-11) |
| [0009](0009-the-cli-asks-the-running-app.md) | The `launcher` binary is also the CLI and asks the running app | Accepted (2026-11-11) |
| [0010](0010-rust-formats-time-and-dates.md) | Times are stored as UTC milliseconds and formatted by the platform | Accepted (2026-11-11) |
| [0011](0011-siblings-talk-through-their-clis.md) | Launcher talks to DataBee and Daybook only through their CLIs | Accepted (2026-11-11); debug DataBee superseded by 0041 |
| [0012](0012-json-files-only-no-database.md) | Launcher stores its data in JSON files only, without a database | Accepted (2026-11-11) |
| [0013](0013-json-stores-repair-themselves-on-load.md) | JSON stores repair themselves on load | Accepted (2026-11-11) |
| [0014](0014-page-reads-go-through-tanstack-query.md) | Page reads go through TanStack Query | Accepted (2026-11-11) |
| [0015](0015-writes-are-operations-with-inverses-for-undo.md) | Writes are operations with inverses, so every change can be undone | Accepted (2026-11-11) |
| [0016](0016-launcher-has-no-network-code.md) | Launcher has no network code | Accepted (2026-11-11); link rule superseded by 0040 |
| [0017](0017-hardened-runtime-strict-csp-and-no-navigation.md) | Hardened runtime, strict CSP and no navigation | Accepted (2026-11-11) |
| [0018](0018-outside-content-is-shown-as-text-and-run-only-by-id.md) | Outside content is shown as text and actions run only by id | Accepted (2026-11-11); link rule superseded by 0040 |
| [0019](0019-builds-are-signed-with-an-apple-development-certificate.md) | Builds are signed with an Apple Development certificate | Accepted (2026-11-11) |
| [0020](0020-every-release-build-gets-a-new-version.md) | Every release build gets a new version | Accepted (2026-11-11); bump rule superseded by 0039 |
| [0021](0021-test-doubles-compile-only-behind-test-util.md) | Test doubles compile only behind test-util | Accepted (2026-11-11) |
| [0022](0022-the-debug-build-is-a-separate-app-on-demo-data.md) | The debug build is a separate app on demo data | Accepted (2026-11-11); DataBee stand-in superseded by 0041 |
| [0023](0023-debug-builds-carry-hooks-for-driving-the-app.md) | Debug builds carry hooks for driving the app without focus | Accepted (2026-11-11) |
| [0024](0024-native-look-with-the-main-window-following-the-prototype.md) | Native look, with the main window following the prototype | Accepted (2026-11-11) |
| [0025](0025-every-color-is-a-palette-token.md) | Every color is a palette token, checked by tests | Accepted (2026-11-11) |
| [0026](0026-automatic-light-or-dark-with-one-palette-each.md) | Automatic, Light or Dark, with one palette each | Accepted (2026-11-11); "no accent choice" superseded by 0042 |
| [0027](0027-native-overlay-scrollbars.md) | Scrollbars are the native overlay bars | Accepted (2026-11-11) |
| [0028](0028-motion-tokens-and-stable-controls.md) | Motion uses shared tokens and controls stay where they are | Accepted (2026-11-11) |
| [0029](0029-one-tooltip-host-per-window.md) | One tooltip host per window replaces native tooltips | Accepted (2026-11-11) |
| [0030](0030-menus-are-drawn-in-the-page.md) | Context menus and pop-up buttons are drawn in the page | Accepted (2026-11-11) |
| [0031](0031-the-palette-runs-the-menus.md) | The command palette runs the menus | Accepted (2026-11-11) |
| [0032](0032-keyboard-first.md) | Every action has a menu item, a palette entry and a rebindable shortcut | Accepted (2026-11-11) |
| [0033](0033-settings-is-its-own-window.md) | Settings is a window of its own in System Settings layout | Accepted (2026-11-11) |
| [0034](0034-a-menu-bar-app-that-keeps-running.md) | Launcher is a menu-bar app; closing a window keeps it running | Accepted (2026-11-11); keys extended by 0048 |
| [0035](0035-no-status-bar.md) | Launcher has no status bar | Accepted (2026-11-11) |
| [0036](0036-global-shortcuts-only-in-release-builds.md) | Global shortcuts are registered only by release builds | Accepted (2026-11-11) |
| [0037](0037-the-palette-is-a-floating-panel-on-a-native-window-layer.md) | The palette is a floating panel on a native window layer | Accepted (2026-11-11) |
| [0038](0038-project-details-live-in-the-project-file.md) | Every project detail lives in the project's own file | Accepted (2026-11-11) |
| [0039](0039-a-major-version-needs-the-owners-approval.md) | A major version needs the owner's approval | Accepted (2026-11-11) |
| [0040](0040-links-open-over-https-or-http-to-a-local-name.md) | Links open over https, or over http to a local name | Accepted (2026-11-12) |
| [0041](0041-debug-builds-record-databee-like-every-launch.md) | Debug builds record DataBee like every other launch | Accepted (2026-11-12) |
| [0042](0042-settings-follow-timetracker-with-an-accent-choice.md) | Settings follow TimeTracker, with an accent choice | Accepted (2026-11-12); groups superseded by 0045 |
| [0043](0043-the-view-is-remembered-outside-undo.md) | The window frame and sidebar choice are remembered outside undo | Accepted (2026-11-12) |
| [0044](0044-a-launch-outside-a-terminal-starts-in-the-menu-bar.md) | A launch outside a terminal starts in the menu bar | Accepted (2026-11-12) |
| [0045](0045-settings-groups-are-taskboards-inset-cards.md) | Settings groups are Taskboard's inset cards; no headline repeats | Accepted (2026-11-12) |
| [0046](0046-settings-search-finds-every-row.md) | The Settings search finds every row | Accepted (2026-11-12) |
| [0047](0047-the-cli-output-siblings-read-is-a-contract.md) | The CLI output siblings read is a contract | Accepted (2026-11-12) |
| [0048](0048-no-single-key-press-quits.md) | No single key press quits; ⌘Q and Esc hide the window | Accepted (2026-11-16) |
