//! Tauri commands for the data: reads, the writes the page sends, undo and redo. Each write announces what
//! it changed as events (ADR 0014); a settings change also refreshes the menu bar and the tray, which rebuild only when their keys or rows changed.

use tauri::{AppHandle, Emitter, Manager, State};

use super::{AddedProjects, Changed, MetaEdit};
use crate::error::{AppError, AppResult};
use crate::events::{PROJECTS_CHANGED_EVENT, SETTINGS_CHANGED_EVENT};
use crate::library::{Filter, Library};
use crate::menu;
use crate::projects::Project;
use crate::projects::config::{ActionDraft, ActionSource, Builtin, DevMode, ProjectConfig, RunsAs};
use crate::state::AppState;
use crate::store::{AppSetting, Appearance, DockerManager, Settings};
use tauri_plugin_dialog::DialogExt;

/// Emits one event per stale read, to every window.
pub fn announce(app: &AppHandle, changed: Changed) {
    if changed.settings {
        menu::refresh(app);
    }
    if changed.settings || changed.projects {
        crate::tray::refresh(app);
    }
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

/// `key: null` leaves the command without a key; a menu item's key needs ⌘ or ⌃.
#[tauri::command]
pub async fn set_app_setting(
    app: AppHandle,
    state: State<'_, AppState>,
    setting: AppSetting,
    value: String,
) -> AppResult<()> {
    announced(&app, state.data.set_app(setting, &value))
}

#[tauri::command]
pub async fn set_docker_manager(
    app: AppHandle,
    state: State<'_, AppState>,
    manager: DockerManager,
) -> AppResult<()> {
    announced(&app, state.data.set_docker_manager(manager))
}

#[tauri::command]
pub async fn set_shortcut(
    app: AppHandle,
    state: State<'_, AppState>,
    command: String,
    key: Option<String>,
) -> AppResult<()> {
    let key = menu::shortcuts::checked(&command, key)?;
    announced(&app, state.data.set_shortcut(&command, key))
}

/// Gives `command` the key `from` has, leaving `from` without one; one step for Undo.
#[tauri::command]
pub async fn reassign_shortcut(
    app: AppHandle,
    state: State<'_, AppState>,
    command: String,
    key: String,
    from: String,
) -> AppResult<()> {
    let key = menu::shortcuts::checked(&command, Some(key))?.unwrap_or_default();
    announced(&app, state.data.reassign_shortcut(&command, &key, &from))
}

/// Back to the default key; `from`, when the default is its key now, is left without one in the same step.
#[tauri::command]
pub async fn reset_shortcut(
    app: AppHandle,
    state: State<'_, AppState>,
    command: String,
    from: Option<String>,
) -> AppResult<()> {
    let changed = match from {
        Some(from) => state.data.reset_shortcut_taking_from(&command, &from),
        None => state.data.reset_shortcut(&command),
    };
    announced(&app, changed)
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

#[tauri::command]
pub async fn library(
    state: State<'_, AppState>,
    filter: Filter,
    query: String,
) -> AppResult<Library> {
    let now = chrono::Utc::now().timestamp_millis();
    Ok(state.data.library(&filter, &query, now))
}

/// Shows the folder panel and adds the chosen folders; nothing when the panel is cancelled.
#[tauri::command]
pub async fn add_project_folders(
    app: AppHandle,
    state: State<'_, AppState>,
) -> AppResult<AddedProjects> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    app.dialog()
        .file()
        .set_title("Add Project")
        .pick_folders(move |paths| {
            let _ = tx.send(paths);
        });
    let chosen = rx
        .await
        .map_err(|_| AppError::Io("The folder panel closed without an answer".into()))?
        .unwrap_or_default();
    let paths = chosen
        .into_iter()
        .filter_map(|path| path.into_path().ok())
        .map(|path| path.to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    let now = chrono::Utc::now().timestamp_millis();
    let (added, changed) = state.data.add_projects(&paths, now)?;
    announce(&app, changed);
    Ok(added)
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
pub async fn remove_project(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> AppResult<()> {
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

#[tauri::command]
pub async fn project_config(state: State<'_, AppState>, id: String) -> AppResult<ProjectConfig> {
    state.data.project_config_shown(&id)
}

/// `source: null` creates the action; `preset` names the built-in the draft overrides.
#[tauri::command]
pub async fn save_action(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    source: Option<ActionSource>,
    draft: ActionDraft,
    preset: Option<Builtin>,
) -> AppResult<()> {
    announced(&app, state.data.save_action(&id, source, &draft, preset))
}

#[tauri::command]
pub async fn duplicate_action(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    source: ActionSource,
    runs_as: Option<RunsAs>,
) -> AppResult<()> {
    announced(&app, state.data.duplicate_action(&id, source, runs_as))
}

#[tauri::command]
pub async fn remove_action(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    source: ActionSource,
) -> AppResult<()> {
    announced(&app, state.data.remove_action(&id, source))
}

#[tauri::command]
pub async fn move_action(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    source: ActionSource,
    to: usize,
) -> AppResult<()> {
    announced(&app, state.data.move_action(&id, source, to))
}

#[tauri::command]
pub async fn set_builtin_enabled(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    preset: Builtin,
    enabled: bool,
) -> AppResult<()> {
    announced(&app, state.data.set_builtin_enabled(&id, preset, enabled))
}

#[tauri::command]
pub async fn set_project_locked(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    locked: bool,
) -> AppResult<()> {
    announced(&app, state.data.set_project_locked(&id, locked))
}

#[tauri::command]
pub async fn set_dev_mode(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    mode: DevMode,
) -> AppResult<()> {
    announced(&app, state.data.set_dev_mode(&id, mode))
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
