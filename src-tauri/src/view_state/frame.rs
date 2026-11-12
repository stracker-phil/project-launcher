//! The main window's frame: noted on every move and resize, saved at most once a second and when the window
//! hides or the app quits, and put back before the window first shows.

use std::time::Duration;

use tauri::{AppHandle, LogicalPosition, LogicalSize, Manager, Runtime, Window};

use super::{Frame, fits};
use crate::state::AppState;
use crate::window;

const SAVE_DELAY: Duration = Duration::from_secs(1);

/// A minimized or full-screen window keeps the frame it had.
pub fn moved_or_resized<R: Runtime>(window: &Window<R>) {
    // A frame placed by scripts/dev isn't the owner's.
    #[cfg(debug_assertions)]
    if crate::dev::window_frame().is_some() {
        return;
    }
    let Some(frame) = frame_of(window) else {
        return;
    };
    let app = window.app_handle().clone();
    if !app.state::<AppState>().view.note_frame(frame) {
        return;
    }
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(SAVE_DELAY).await;
        save(&app);
    });
}

fn frame_of<R: Runtime>(window: &Window<R>) -> Option<Frame> {
    if window.is_minimized().unwrap_or(true) || window.is_fullscreen().unwrap_or(true) {
        return None;
    }
    let scale = window.scale_factor().ok()?;
    let position = window.outer_position().ok()?.to_logical::<f64>(scale);
    let size = window.inner_size().ok()?.to_logical::<f64>(scale);
    Frame {
        x: position.x,
        y: position.y,
        width: size.width,
        height: size.height,
    }
    .checked()
}

/// Writes the frame noted last; a failed write only means the next start uses an older frame.
pub fn save<R: Runtime>(app: &AppHandle<R>) {
    if let Err(e) = app.state::<AppState>().view.save_frame() {
        log::warn!("Couldn't save the window's frame: {e}");
    }
}

/// Puts the main window where it was, if that is still mostly on a connected screen.
pub fn restore<R: Runtime>(app: &AppHandle<R>) {
    let (Some(frame), Some(window)) = (
        app.state::<AppState>().view.saved_frame(),
        app.get_webview_window(window::MAIN),
    ) else {
        return;
    };
    let screens: Vec<Frame> = window
        .available_monitors()
        .unwrap_or_default()
        .iter()
        .map(|monitor| {
            let scale = monitor.scale_factor();
            let position = monitor.position().to_logical::<f64>(scale);
            let size = monitor.size().to_logical::<f64>(scale);
            Frame {
                x: position.x,
                y: position.y,
                width: size.width,
                height: size.height,
            }
        })
        .collect();
    if !fits(&frame, &screens) {
        log::info!("The saved window frame is off screen; using the default");
        return;
    }
    let _ = window.set_size(LogicalSize::new(frame.width, frame.height));
    let _ = window.set_position(LogicalPosition::new(frame.x, frame.y));
}
