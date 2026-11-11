//! Launch at login: a LaunchAgent written by `tauri-plugin-autostart` that starts the app with `--hidden`
//! (ADR 0034). The LaunchAgent is the truth. Release builds only; the debug app never installs one.

use tauri::AppHandle;

use crate::error::{AppError, AppResult};

#[cfg(not(debug_assertions))]
pub fn plugin() -> tauri::plugin::TauriPlugin<tauri::Wry> {
    tauri_plugin_autostart::init(
        tauri_plugin_autostart::MacosLauncher::LaunchAgent,
        Some(vec!["--hidden"]),
    )
}

#[cfg(not(debug_assertions))]
fn enabled(app: &AppHandle) -> bool {
    use tauri_plugin_autostart::ManagerExt;
    app.autolaunch().is_enabled().unwrap_or(false)
}

#[cfg(debug_assertions)]
fn enabled(_: &AppHandle) -> bool {
    false
}

#[cfg(not(debug_assertions))]
fn apply(app: &AppHandle, on: bool) -> AppResult<()> {
    use tauri_plugin_autostart::ManagerExt;
    let autolaunch = app.autolaunch();
    let result = if on {
        autolaunch.enable()
    } else {
        autolaunch.disable()
    };
    result.map_err(|e| AppError::Io(format!("Couldn't change Open at Login: {e}")))
}

#[cfg(debug_assertions)]
fn apply(_: &AppHandle, _: bool) -> AppResult<()> {
    Err(AppError::InvalidInput(
        "Only the installed app opens at login".into(),
    ))
}

#[tauri::command]
pub fn autostart_enabled(app: AppHandle) -> bool {
    enabled(&app)
}

/// Returns whether it is on now.
#[tauri::command]
pub fn set_autostart(app: AppHandle, on: bool) -> AppResult<bool> {
    if enabled(&app) != on {
        apply(&app, on)?;
    }
    Ok(enabled(&app))
}
