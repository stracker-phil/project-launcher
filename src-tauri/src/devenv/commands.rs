//! Tauri commands for the dev environment. Steps run on a thread of their own; what is under way changes
//! announce `dev_changed`, a failed step `dev_failed` with its output.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use ts_rs::TS;

use super::{DevAction, DevStatus, Failure, Target};
use crate::error::{AppError, AppResult};
use crate::events::{DEV_CHANGED_EVENT, DEV_FAILED_EVENT};
use crate::projects::FileRead;
use crate::state::AppState;

static APP: OnceLock<AppHandle> = OnceLock::new();

/// Lets the dev environment announce its changes; once, at startup.
pub fn attach(app: &AppHandle) {
    let _ = APP.set(app.clone());
}

/// `dev_changed` to every window.
pub fn announce_change() {
    if let Some(app) = APP.get()
        && let Err(e) = app.emit(DEV_CHANGED_EVENT, ())
    {
        log::warn!("Couldn't announce a dev environment change: {e}");
    }
}

/// A failed step, for the project's detail.
#[derive(Serialize, TS, Debug, Clone)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct DevFailure {
    pub project_id: String,
    #[serde(flatten)]
    #[ts(flatten)]
    pub failure: Failure,
}

fn report(app: &AppHandle, project_id: &str, failure: Failure) {
    log::warn!("{} failed: {}", failure.label, failure.output);
    let payload = DevFailure {
        project_id: project_id.into(),
        failure,
    };
    if let Err(e) = app.emit(DEV_FAILED_EVENT, payload) {
        log::warn!("Couldn't report a dev environment failure: {e}");
    }
}

/// The project as the dev environment needs it.
pub fn target(state: &AppState, id: &str) -> AppResult<Target> {
    let config = state.data.project_config(id)?;
    let dir = PathBuf::from(state.data.project_dir(id)?);
    let database = match crate::projects::file::read(&dir) {
        FileRead::Found(object) => object.get("database").cloned(),
        FileRead::Missing | FileRead::Broken(_) => None,
    };
    Ok(Target {
        id: id.into(),
        name: state.data.project(id)?.name,
        env: config
            .env
            .iter()
            .map(|var| (var.name.clone(), var.value.clone()))
            .collect(),
        mode: config.dev,
        database,
        dir,
    })
}

/// Every project that can be a target; one whose file is missing or broken has no server to ask about.
fn all_targets(state: &AppState) -> Vec<Target> {
    state
        .data
        .projects()
        .iter()
        .filter_map(|p| target(state, &p.id).ok())
        .collect()
}

/// Runs `work` on a thread of its own; a failure goes to the page as `dev_failed`.
fn in_background(
    app: &AppHandle,
    id: String,
    work: impl FnOnce(&AppState, &Target) -> Result<(), Failure> + Send + 'static,
) {
    let app = app.clone();
    std::thread::spawn(move || {
        let state = app.state::<AppState>();
        let result = target(&state, &id)
            .map_err(|e| Failure::new("Dev environment", e.to_string()))
            .and_then(|target| work(&state, &target));
        if let Err(failure) = result {
            report(&app, &id, failure);
        }
    });
}

/// Starts the project's environment when something of it opens, once per session.
pub fn prepare_in_background(app: &AppHandle, id: &str) {
    in_background(app, id.into(), |state, target| {
        state.devenv.prepare(target, &state.data.settings())
    });
}

/// The project's dev status, asked off the async runtime.
pub async fn status_of(app: AppHandle, id: String) -> AppResult<DevStatus> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        Ok(state
            .devenv
            .status(&target(&state, &id)?, &state.data.settings()))
    })
    .await
    .map_err(|e| AppError::Internal(e.to_string()))?
}

#[tauri::command]
pub async fn dev_status(app: AppHandle, id: String) -> AppResult<DevStatus> {
    status_of(app, id).await
}

/// The projects whose dev server runs, for the list's dots.
#[tauri::command]
pub async fn dev_running(app: AppHandle) -> AppResult<Vec<String>> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        Ok(state.devenv.running(&all_targets(&state)))
    })
    .await
    .map_err(|e| AppError::Internal(e.to_string()))?
}

/// Runs a start, stop or restart on a thread of its own: the detail's buttons and the `dev:` commands.
pub fn act_in_background(app: &AppHandle, id: String, action: DevAction) {
    in_background(app, id, move |state, target| {
        state.devenv.act(target, action, &state.data.settings())
    });
}

#[tauri::command]
pub fn dev_action(app: AppHandle, id: String, action: DevAction) {
    act_in_background(&app, id, action);
}

/// `starting`: the database's server starts first; DataBee opens once it runs.
#[derive(Serialize, TS, Debug, Clone, Copy, PartialEq, Eq)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub enum OpenedDatabase {
    Opened,
    Starting,
}

fn open(state: &AppState, target: &Target, home: Option<&Path>) -> Result<(), Failure> {
    let launch = state
        .devenv
        .database_launch(target, &state.data.settings(), home)?;
    state
        .system
        .spawn(&launch.job())
        .map_err(|e| Failure::new("Open Database", e.to_string()))
}

/// Opens the project's database in DataBee; a dev server that doesn't run yet is started first, in the background.
#[tauri::command]
pub async fn open_database(app: AppHandle, id: String) -> AppResult<OpenedDatabase> {
    let home = app.path().home_dir().ok();
    let later = home.clone();
    let handle = app.clone();
    let project = id.clone();
    let ready = tauri::async_runtime::spawn_blocking(move || -> AppResult<bool> {
        let state = handle.state::<AppState>();
        let target = target(&state, &project)?;
        if !state.devenv.database_ready(&target) {
            return Ok(false);
        }
        open(&state, &target, home.as_deref()).map_err(|f| AppError::Launch(f.output))?;
        Ok(true)
    })
    .await
    .map_err(|e| AppError::Internal(e.to_string()))??;
    if ready {
        return Ok(OpenedDatabase::Opened);
    }
    in_background(&app, id, move |state, target| {
        open(state, target, later.as_deref())
    });
    Ok(OpenedDatabase::Starting)
}
