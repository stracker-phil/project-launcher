//! Tauri commands for launching. They wait on processes, so they run off the async runtime; each one that
//! opens something announces the project list (its last-opened time changed).

use tauri::{AppHandle, Manager};

use super::{Outcome, ProjectStatus, Runner};
use crate::data::Changed;
use crate::data::commands::announce;
use crate::error::{AppError, AppResult};
use crate::projects::config::ActionSource;
use crate::state::AppState;

/// Runs `work` with a runner on a blocking thread.
async fn with_runner<T: Send + 'static>(
    app: &AppHandle,
    work: impl FnOnce(&Runner, i64) -> AppResult<T> + Send + 'static,
) -> AppResult<T> {
    let handle = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let state = handle.state::<AppState>();
        work(&state.runner(), chrono::Utc::now().timestamp_millis())
    })
    .await
    .map_err(|e| AppError::Internal(e.to_string()))?
}

/// Runs `work` like `with_runner`; once it went through, announces the changed last-opened time.
async fn opening<T: Send + 'static>(
    app: AppHandle,
    work: impl FnOnce(&Runner, i64) -> AppResult<T> + Send + 'static,
) -> AppResult<T> {
    let result = with_runner(&app, work).await?;
    announce(
        &app,
        Changed {
            settings: false,
            projects: true,
        },
    );
    Ok(result)
}

#[tauri::command]
pub async fn launch_action(app: AppHandle, id: String, source: ActionSource) -> AppResult<Outcome> {
    crate::devenv::commands::prepare_in_background(&app, &id);
    opening(app, move |runner, now| runner.launch(&id, source, now)).await
}

#[tauri::command]
pub async fn open_terminal(app: AppHandle, id: String) -> AppResult<()> {
    crate::devenv::commands::prepare_in_background(&app, &id);
    opening(app, move |runner, now| runner.open_terminal(&id, now)).await
}

#[tauri::command]
pub async fn reveal_project(app: AppHandle, id: String) -> AppResult<()> {
    opening(app, move |runner, now| runner.reveal(&id, now)).await
}

#[tauri::command]
pub async fn copy_project_path(app: AppHandle, id: String) -> AppResult<()> {
    with_runner(&app, move |runner, _| runner.copy_path(&id)).await
}

#[tauri::command]
pub async fn edit_project_file(app: AppHandle, id: String) -> AppResult<()> {
    opening(app, move |runner, now| runner.edit_file(&id, now)).await
}

#[tauri::command]
pub async fn project_status(app: AppHandle, id: String) -> AppResult<ProjectStatus> {
    with_runner(&app, move |runner, _| runner.status(&id)).await
}
