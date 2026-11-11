//! Every event the backend emits to the page, with its payload type.

/// No payload; the page then calls `take_launch_events`.
pub const LAUNCH_EVENT: &str = "launch";
/// No payload; the hidden main window was shown again.
pub const WINDOW_OPENED_EVENT: &str = "window_opened";
/// The id of a page command (`page:<id>` without the prefix) for the main window to run.
pub const PAGE_COMMAND_EVENT: &str = "page_command";
/// The settings page (or an unknown id, which shows the first page) for the Settings window to show.
pub const OPEN_SETTINGS_EVENT: &str = "open_settings";
