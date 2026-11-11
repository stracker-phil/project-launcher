//! Revealing the app's own folders in Finder (Help menu).

use std::path::PathBuf;

use tauri::{AppHandle, Manager};
use tauri_plugin_opener::OpenerExt;

use crate::error::{AppError, AppResult};

pub fn open_log_folder(app: &AppHandle) -> AppResult<()> {
    open(app, app.path().app_log_dir())
}

/// The config folder with `settings.json` and `projects.json`.
pub fn open_data_folder(app: &AppHandle) -> AppResult<()> {
    open(app, app.path().app_config_dir())
}

fn open(app: &AppHandle, dir: tauri::Result<PathBuf>) -> AppResult<()> {
    let dir = dir.map_err(|e| AppError::Io(e.to_string()))?;
    app.opener()
        .open_path(dir.to_string_lossy(), None::<&str>)
        .map_err(|e| AppError::Io(e.to_string()))
}
