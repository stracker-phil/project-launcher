//! The command line's project requests: each resolves its project (and action) by name, then does what the page's
//! buttons do.

use std::path::Path;

use serde::Serialize;
use tauri::{AppHandle, Manager};

use crate::automation::{self, import};
use crate::bus;
use crate::data::Changed;
use crate::data::commands::announce;
use crate::error::{AppError, AppResult};
use crate::ipc::{HandlerResult, IpcMessage};
use crate::projects::config::resolve::Action;
use crate::run::Outcome;
use crate::state::AppState;
use crate::window;

fn json(value: impl Serialize) -> HandlerResult {
    serde_json::to_value(value).map_err(|e| AppError::from(e).into())
}

fn done() -> HandlerResult {
    Ok(serde_json::Value::Null)
}

/// Runs `work` with the app state and the time off the async runtime.
async fn blocking<T: Send + 'static>(
    app: &AppHandle,
    work: impl FnOnce(&AppState, i64) -> AppResult<T> + Send + 'static,
) -> AppResult<T> {
    let handle = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        work(
            &handle.state::<AppState>(),
            chrono::Utc::now().timestamp_millis(),
        )
    })
    .await
    .map_err(|e| AppError::Internal(e.to_string()))?
}

/// The project `spec` names, resolved off the async runtime.
async fn resolve(app: &AppHandle, spec: String) -> AppResult<String> {
    blocking(app, move |state, _| automation::resolve(&state.data, &spec)).await
}

/// Runs `work` on the project `spec` names; it counts as opened, so the list's recent order updates.
async fn on_project<T: Send + 'static>(
    app: &AppHandle,
    spec: String,
    work: impl FnOnce(&AppState, &str, i64) -> AppResult<T> + Send + 'static,
) -> AppResult<T> {
    let result = blocking(app, move |state, now| {
        work(state, &automation::resolve(&state.data, &spec)?, now)
    })
    .await;
    announce(
        app,
        Changed {
            settings: false,
            projects: true,
        },
    );
    result
}

/// Launches the project's app or script with that label.
async fn launch(
    app: &AppHandle,
    spec: String,
    name: String,
    kind: &'static str,
) -> AppResult<Outcome> {
    on_project(app, spec, move |state, id, now| {
        let config = state.data.project_config(id)?;
        let actions: &[Action] = if kind == "app" {
            &config.apps
        } else {
            &config.scripts
        };
        let action = automation::pick(actions, &name, kind, &state.data.project(id)?.name)?;
        state.runner().launch(id, action.edit.source, now)
    })
    .await
}

async fn run_command(app: &AppHandle, id: &str) -> AppResult<()> {
    let work = app.state::<bus::Commands>().start(app.clone(), id)?;
    work.await
}

async fn reply<T: Serialize>(result: impl Future<Output = AppResult<T>>) -> HandlerResult {
    result.await.map_err(Into::into).and_then(json)
}

/// `None` when the message isn't one of the command line's project requests.
pub async fn handle(app: &AppHandle, message: &IpcMessage) -> Option<HandlerResult> {
    let app = app.clone();
    Some(match message.clone() {
        IpcMessage::List => json(automation::list(&app.state::<AppState>().data)),
        IpcMessage::Describe { project } => {
            reply(blocking(&app, move |state, _| {
                automation::describe(&state.data, &automation::resolve(&state.data, &project)?)
            }))
            .await
        }
        IpcMessage::Commands => json(app.state::<bus::Commands>().listed()),
        IpcMessage::Open { project } => {
            reply(async {
                let id = resolve(&app, project).await?;
                run_command(&app, &crate::run::bus::show_command(&id)).await
            })
            .await
        }
        IpcMessage::Launch { project, name } => {
            reply(async { launch(&app, project, name, "app").await.map(|_| ()) }).await
        }
        IpcMessage::Run { project, name } => match launch(&app, project, name, "script").await {
            Ok(Outcome::Ran { result }) => match result.failure() {
                Some(failure) => Err(failure.into()),
                None => json(result),
            },
            Ok(Outcome::Opened { .. }) => done(),
            Err(e) => Err(e.into()),
        },
        IpcMessage::Terminal { project } => {
            reply(on_project(&app, project, |state, id, now| {
                state.runner().open_terminal(id, now)
            }))
            .await
        }
        IpcMessage::Finder { project } => {
            reply(on_project(&app, project, |state, id, now| {
                state.runner().reveal(id, now)
            }))
            .await
        }
        IpcMessage::Config { project } => {
            reply(on_project(&app, project, |state, id, now| {
                state.runner().edit_file(id, now)
            }))
            .await
        }
        IpcMessage::Db { project } => {
            reply(async {
                let id = resolve(&app, project).await?;
                crate::devenv::commands::open_database(app.clone(), id).await
            })
            .await
        }
        IpcMessage::Palette => {
            window::palette::toggle(&app);
            done()
        }
        IpcMessage::Settings => {
            window::settings::open(&app, None);
            done()
        }
        IpcMessage::Command { id } => reply(run_command(&app, &id)).await,
        IpcMessage::Import { folder, yes } => {
            let result = blocking(&app, move |state, _| {
                import::run(&state.data, Path::new(&folder), yes)
            })
            .await;
            match result {
                Ok((plan, changed)) => {
                    if let Some(changed) = changed {
                        announce(&app, changed);
                    }
                    json(plan)
                }
                Err(e) => Err(e.into()),
            }
        }
        _ => return None,
    })
}
