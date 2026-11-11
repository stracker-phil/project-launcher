//! Tauri commands for the data: reads, the writes the page sends, undo and redo. Each write announces what
//! it changed as events (ADR 0014).

use tauri::{AppHandle, Emitter, Manager, State};

use super::{Changed, MetaEdit};
use crate::error::AppResult;
use crate::events::{PROJECTS_CHANGED_EVENT, SETTINGS_CHANGED_EVENT};
use crate::projects::Project;
use crate::state::AppState;
use crate::store::{Appearance, Settings};

/// Emits one event per stale read, to every window.
pub fn announce(app: &AppHandle, changed: Changed) {
    for (stale, event) in [
        (changed.settings, SETTINGS_CHANGED_EVENT),
        (changed.projects, PROJECTS_CHANGED_EVENT),
    ] {
        if stale && let Err(e) = app.emit(event, ()) {
            log::warn!("Couldn't announce {event}: {e}");
        }
    }
}

fn announced(app: &AppHandle, changed: AppResult<Changed>) -> AppResult<()> {
    announce(app, changed?);
    Ok(())
}

/// Takes back the last change and returns its label; `None` when there is nothing to undo.
pub fn undo(app: &AppHandle) -> AppResult<Option<String>> {
    let reverted = app.state::<AppState>().data.undo()?;
    Ok(reverted.map(|r| {
        announce(app, r.changed);
        r.label
    }))
}

pub fn redo(app: &AppHandle) -> AppResult<Option<String>> {
    let reverted = app.state::<AppState>().data.redo()?;
    Ok(reverted.map(|r| {
        announce(app, r.changed);
        r.label
    }))
}

#[tauri::command]
pub async fn get_settings(state: State<'_, AppState>) -> AppResult<Settings> {
    Ok(state.data.settings())
}

#[tauri::command]
pub async fn list_projects(state: State<'_, AppState>) -> AppResult<Vec<Project>> {
    Ok(state.data.projects())
}

/// Reads every project file from disk again.
#[tauri::command]
pub async fn reload_projects(app: AppHandle, state: State<'_, AppState>) -> AppResult<()> {
    state.data.reload();
    announce(
        &app,
        Changed {
            settings: false,
            projects: true,
        },
    );
    Ok(())
}

#[tauri::command]
pub async fn set_appearance(
    app: AppHandle,
    state: State<'_, AppState>,
    appearance: Appearance,
) -> AppResult<()> {
    announced(&app, state.data.set_appearance(appearance))
}

/// `key: null` leaves the command without a key.
#[tauri::command]
pub async fn set_shortcut(
    app: AppHandle,
    state: State<'_, AppState>,
    command: String,
    key: Option<String>,
) -> AppResult<()> {
    announced(&app, state.data.set_shortcut(&command, key))
}

#[tauri::command]
pub async fn reset_shortcut(
    app: AppHandle,
    state: State<'_, AppState>,
    command: String,
) -> AppResult<()> {
    announced(&app, state.data.reset_shortcut(&command))
}

#[tauri::command]
pub async fn add_tag(app: AppHandle, state: State<'_, AppState>, name: String) -> AppResult<()> {
    announced(&app, state.data.add_tag(&name))
}

#[tauri::command]
pub async fn rename_tag(
    app: AppHandle,
    state: State<'_, AppState>,
    from: String,
    to: String,
) -> AppResult<()> {
    announced(&app, state.data.rename_tag(&from, &to))
}

#[tauri::command]
pub async fn delete_tag(app: AppHandle, state: State<'_, AppState>, name: String) -> AppResult<()> {
    announced(&app, state.data.delete_tag(&name))
}

#[tauri::command]
pub async fn set_tag_order(
    app: AppHandle,
    state: State<'_, AppState>,
    tags: Vec<String>,
) -> AppResult<()> {
    announced(&app, state.data.set_tag_order(tags))
}

/// Returns the added project.
#[tauri::command]
pub async fn add_project(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> AppResult<Project> {
    let now = chrono::Utc::now().timestamp_millis();
    let (id, changed) = state.data.add_project(&path, now)?;
    announce(&app, changed);
    state.data.project(&id)
}

#[tauri::command]
pub async fn remove_project(app: AppHandle, state: State<'_, AppState>, id: String) -> AppResult<()> {
    announced(&app, state.data.remove_project(&id))
}

#[tauri::command]
pub async fn set_project_name(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    name: String,
) -> AppResult<()> {
    announced(&app, state.data.set_project_name(&id, &name))
}

#[tauri::command]
pub async fn set_project_meta(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    edit: MetaEdit,
) -> AppResult<()> {
    announced(&app, state.data.set_project_meta(&id, edit))
}

/// The label of the undone change, or `null`.
#[tauri::command]
pub async fn undo_change(app: AppHandle) -> AppResult<Option<String>> {
    undo(&app)
}

#[tauri::command]
pub async fn redo_change(app: AppHandle) -> AppResult<Option<String>> {
    redo(&app)
}
