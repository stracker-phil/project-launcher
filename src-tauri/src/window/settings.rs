//! The Settings window (ADR 0033): built on first use and hidden on close, so it reopens on the page it showed.

use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

use crate::events::OPEN_SETTINGS_EVENT;

pub const LABEL: &str = "settings";

/// The settings page, opened at `page` (`?page=`) when given.
pub fn url(page: Option<&str>) -> String {
    let query = page
        .map(|page| {
            let encoded: String = url::form_urlencoded::byte_serialize(page.as_bytes()).collect();
            // A literal `+` is already `%2B`, so every `+` left stands for a space.
            format!("?page={}", encoded.replace('+', "%20"))
        })
        .unwrap_or_default();
    format!("src/windows/settings/index.html{query}")
}

fn build(app: &AppHandle, page: Option<&str>) -> tauri::Result<WebviewWindow> {
    WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App(url(page).into()))
        .title("Settings")
        .inner_size(880.0, 720.0)
        .min_inner_size(760.0, 560.0)
        .maximizable(false)
        .visible(false)
        .title_bar_style(tauri::TitleBarStyle::Overlay)
        .hidden_title(true)
        .traffic_light_position(tauri::LogicalPosition::new(20.0, 22.0))
        .build()
}

/// Tells the open window to show `page`.
fn navigate(window: &WebviewWindow, page: &str) {
    if let Err(e) = window.emit_to(LABEL, OPEN_SETTINGS_EVENT, page) {
        log::warn!("Couldn't show the Settings page {page}: {e}");
    }
}

/// Shows Settings at `page` when given (else where it was) and brings it forward.
pub fn open(app: &AppHandle, page: Option<&str>) {
    let window = match app.get_webview_window(LABEL) {
        Some(window) => {
            if let Some(page) = page {
                navigate(&window, page);
            }
            window
        }
        None => match build(app, page) {
            Ok(window) => window,
            Err(e) => {
                log::warn!("Couldn't open Settings: {e}");
                return;
            }
        },
    };
    super::set_dock_icon(app, true);
    if let Err(e) = window.show() {
        log::warn!("Couldn't show Settings: {e}");
    }
    super::bring_to_front(&window);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_url_carries_a_page_only_when_given() {
        for (page, expected) in [
            (None, "src/windows/settings/index.html"),
            (Some("status"), "src/windows/settings/index.html?page=status"),
            (Some("a b&c"), "src/windows/settings/index.html?page=a%20b%26c"),
        ] {
            assert_eq!(url(page), expected, "{page:?}");
        }
    }
}
