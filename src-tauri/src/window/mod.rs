//! The app's windows. Closing one hides it; the app keeps running in the menu bar until it quits, and the Dock
//! icon shows only while a regular window is open (ADR 0034).

pub mod navigation;
pub mod settings;

use tauri::{AppHandle, Emitter, Manager, WebviewWindow};

use crate::events::{PAGE_COMMAND_EVENT, WINDOW_OPENED_EVENT};

/// The main window's label.
pub const MAIN: &str = "main";

/// Windows that give the app a Dock icon while they show; the palette panel never does.
const REGULAR: [&str; 2] = [MAIN, settings::LABEL];

/// Unminimizes and focuses the window; skipped with `LAUNCHER_BACKGROUND=1` in debug builds so
/// scripts/dev never steal focus.
fn bring_to_front(window: &WebviewWindow) {
    #[cfg(debug_assertions)]
    if crate::dev::background() {
        return;
    }
    // Failing only means the window is gone; there is nothing left to bring forward.
    let _ = window.unminimize();
    let _ = window.set_focus();
}

/// Shows the hidden or minimized main window and brings it to the front.
pub fn show_main(app: &AppHandle) {
    show_main_in_background(app);
    if let Some(window) = app.get_webview_window(MAIN) {
        bring_to_front(&window);
    }
}

/// Shows the main window without bringing the app to the front; emits `WINDOW_OPENED_EVENT` when it
/// was hidden.
pub fn show_main_in_background(app: &AppHandle) {
    let opening = !main_visible(app);
    set_dock_icon(app, true);
    if let Some(window) = app.get_webview_window(MAIN)
        && let Err(e) = window.show()
    {
        log::warn!("Couldn't show the Launcher window: {e}");
    }
    if opening && let Err(e) = app.emit(WINDOW_OPENED_EVENT, ()) {
        log::warn!("Couldn't tell the page the window opened: {e}");
    }
}

fn is_visible(app: &AppHandle, label: &str) -> bool {
    app.get_webview_window(label)
        .and_then(|window| window.is_visible().ok())
        .unwrap_or(false)
}

pub fn main_visible(app: &AppHandle) -> bool {
    is_visible(app, MAIN)
}

/// Hides a regular window. With the last one gone the Dock icon goes too, and the app steps back so the
/// app behind it gets focus.
fn hide_regular(app: &AppHandle, label: &str) {
    if let Some(window) = app.get_webview_window(label) {
        hide_or_warn(&window);
    }
    if REGULAR.iter().any(|label| is_visible(app, label)) {
        return;
    }
    set_dock_icon(app, false);
    #[cfg(target_os = "macos")]
    if let Err(e) = app.hide() {
        log::warn!("Couldn't hand focus back: {e}");
    }
}

pub fn hide_main(app: &AppHandle) {
    hide_regular(app, MAIN);
}

/// Hides a regular window on close and returns true, so the close doesn't proceed.
pub fn close_requested(app: &AppHandle, label: &str) -> bool {
    let ours = REGULAR.contains(&label);
    if ours {
        hide_regular(app, label);
    }
    ours
}

fn focused_window(app: &AppHandle) -> Option<WebviewWindow> {
    app.webview_windows()
        .into_values()
        .find(|window| window.is_focused().unwrap_or(false))
}

/// ⌘W: hides the window in front; a regular window hides as its close button does.
pub fn close_focused(app: &AppHandle) {
    let Some(window) = focused_window(app) else {
        return;
    };
    if !close_requested(app, window.label()) {
        hide_or_warn(&window);
    }
}

fn hide_or_warn(window: &WebviewWindow) {
    if let Err(e) = window.hide() {
        log::warn!("Couldn't hide the {} window: {e}", window.label());
    }
}

/// Shows the main window and asks its page to run one of its own commands (`page:<id>` from the menus).
pub fn run_page_command(app: &AppHandle, id: &str) {
    show_main(app);
    if let Err(e) = app.emit_to(MAIN, PAGE_COMMAND_EVENT, id) {
        log::warn!("Couldn't ask the window to run {id}: {e}");
    }
}

#[cfg(debug_assertions)]
pub fn reload_focused(app: &AppHandle) {
    if let Some(window) = focused_window(app)
        && let Err(e) = window.eval("location.reload()")
    {
        log::warn!("Couldn't reload the page: {e}");
    }
}

pub fn set_dock_icon(app: &AppHandle, shown: bool) {
    #[cfg(target_os = "macos")]
    if let Err(e) = app.set_dock_visibility(shown) {
        log::warn!("Couldn't change the Dock icon: {e}");
    }
}
