//! Tauri commands for the inspector. Scans read the disk, so they run off the async runtime.

use std::path::PathBuf;

use tauri::{AppHandle, Manager};

use super::manifests;
use super::{FileStats, QuickStats};
use crate::error::{AppError, AppResult};
use crate::run::terminal::Launched;
use crate::state::AppState;

async fn blocking<T: Send + 'static>(
    app: AppHandle,
    work: impl FnOnce(&AppState, i64) -> AppResult<T> + Send + 'static,
) -> AppResult<T> {
    tauri::async_runtime::spawn_blocking(move || {
        work(
            &app.state::<AppState>(),
            chrono::Utc::now().timestamp_millis(),
        )
    })
    .await
    .map_err(|e| AppError::Internal(e.to_string()))?
}

#[tauri::command]
pub async fn quick_stats(app: AppHandle, id: String, refresh: bool) -> AppResult<QuickStats> {
    blocking(app, move |state, now| {
        let dir = PathBuf::from(state.data.project_dir(&id)?);
        Ok(state.stats.quick(&id, &dir, now, refresh))
    })
    .await
}

#[tauri::command]
pub async fn file_stats(app: AppHandle, id: String, refresh: bool) -> AppResult<FileStats> {
    blocking(app, move |state, now| {
        let dir = PathBuf::from(state.data.project_dir(&id)?);
        Ok(state.stats.files(&id, &dir, now, refresh))
    })
    .await
}

/// Runs a script the project's manifest lists in a terminal; one still running there comes to the front.
#[tauri::command]
pub async fn run_package_script(
    app: AppHandle,
    id: String,
    manager: manifests::Manager,
    name: String,
) -> AppResult<Launched> {
    blocking(app, move |state, now| {
        let dir = PathBuf::from(state.data.project_dir(&id)?);
        let listed = manifests::facts(&dir)
            .scripts
            .into_iter()
            .find(|s| s.manager == manager && s.name == name)
            .ok_or_else(|| AppError::NotFound(format!("The manifest has no script {name}")))?;
        state.runner().terminal_command(&id, &listed.command, now)
    })
    .await
}

/// Opens one of the project's files in its editor.
#[tauri::command]
pub async fn open_project_file(app: AppHandle, id: String, path: String) -> AppResult<()> {
    blocking(app, move |state, now| {
        state.runner().open_file(&id, &path, now)
    })
    .await
}
