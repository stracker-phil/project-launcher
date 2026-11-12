//! Tauri commands of the app core; feature modules keep theirs in their own `commands.rs`.

use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;

use crate::error::{AppError, AppResult};
use crate::launch::LaunchEvent;
use crate::links;
use crate::state::AppState;

/// Opens a link in the default browser; the webview itself never navigates away.
#[tauri::command]
pub fn open_url(app: AppHandle, url: String) -> AppResult<()> {
    if !links::is_web_url(&url) {
        return Err(AppError::InvalidInput(format!("Not a web link: {url}")));
    }
    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|e| AppError::Io(e.to_string()))
}

/// The page listens: it takes what waited for it.
#[tauri::command]
pub fn take_launch_events(state: State<'_, AppState>) -> Vec<LaunchEvent> {
    state.launches.take_events()
}

#[tauri::command]
pub fn log_frontend_error(message: String) {
    log::error!(target: "frontend", "{}", sanitize_log_text(&message));
}

const MAX_LOGGED_BYTES: usize = 4096;

/// Bounds page-supplied text and escapes line breaks, so it cannot flood the log or forge log lines.
pub fn sanitize_log_text(text: &str) -> String {
    text[..text.floor_char_boundary(MAX_LOGGED_BYTES)]
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log_text_escapes_line_breaks() {
        assert_eq!(sanitize_log_text("a\nb\r\nc"), "a\\nb\\r\\nc");
    }

    #[test]
    fn short_log_text_is_unchanged() {
        assert_eq!(sanitize_log_text("plain text"), "plain text");
    }

    #[test]
    fn log_text_is_capped_at_4096_bytes() {
        let text = "a".repeat(5000);
        assert_eq!(sanitize_log_text(&text), "a".repeat(4096));
    }

    #[test]
    fn log_text_is_cut_on_a_char_boundary() {
        let text = format!("{}{}", "a".repeat(4095), "é".repeat(10));
        assert_eq!(sanitize_log_text(&text), "a".repeat(4095));
    }
}

/// The main window's page reports the project it shows; the palette and the tray act on it.
#[tauri::command]
pub fn set_current_project(
    app: tauri::AppHandle,
    state: tauri::State<'_, crate::state::AppState>,
    id: Option<String>,
) {
    if state.set_current_project(id) {
        crate::tray::refresh(&app);
    }
}
