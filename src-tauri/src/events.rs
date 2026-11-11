//! Every event the backend emits to the page, with its payload type.

/// No payload; the page then calls `take_launch_events`.
pub const LAUNCH_EVENT: &str = "launch";
/// No payload; the hidden main window was shown again.
pub const WINDOW_OPENED_EVENT: &str = "window_opened";
/// The id of a page command (`page:<id>` without the prefix) for the main window to run.
pub const PAGE_COMMAND_EVENT: &str = "page_command";
/// The settings page (or an unknown id, which shows the first page) for the Settings window to show.
pub const OPEN_SETTINGS_EVENT: &str = "open_settings";
/// No payload; the project list or a project's details changed (`["projects"]`).
pub const PROJECTS_CHANGED_EVENT: &str = "projects_changed";
/// No payload; the settings or the tag list changed (`["settings"]`).
pub const SETTINGS_CHANGED_EVENT: &str = "settings_changed";
/// `"undo"` or `"redo"` from the Edit menu, for the focused window: a text field undoes its typing, anything
/// else undoes the last change of the data.
pub const EDIT_REQUEST_EVENT: &str = "edit_request";
/// The project the main window shows (`null`: none), as the palette opens; the palette page resets and reports
/// back with `palette_shown`.
pub const PALETTE_OPENED_EVENT: &str = "palette_opened";
/// No payload; the palette closed.
pub const PALETTE_CLOSED_EVENT: &str = "palette_closed";
/// `{ id, filter }`: a project for the main window to select, under the filter that shows it (from the palette, the tray or the CLI).
pub const SELECT_PROJECT_EVENT: &str = "select_project";
