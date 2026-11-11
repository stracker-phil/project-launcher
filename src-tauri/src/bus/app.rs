//! The bus inside the app: the app is its host, menus and the tray spawn commands, the page runs them with
//! `run_command`.

use serde::Serialize;
use tauri::{AppHandle, Manager};

use super::{CommandBus, Host};
use crate::error::{AppError, AppResult};

/// The app's commands, registered once at startup.
pub struct Commands(pub CommandBus<AppHandle>);

impl std::ops::Deref for Commands {
    type Target = CommandBus<AppHandle>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// A command as `launcher commands` lists it.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListedCommand {
    pub id: String,
    pub title: String,
    pub help: String,
}

impl Commands {
    /// The commands help shows, in their order.
    pub fn listed(&self) -> Vec<ListedCommand> {
        self.list()
            .filter_map(|c| {
                let info = c.info();
                (!info.hidden).then(|| ListedCommand {
                    id: c.id().into(),
                    title: info.title.into(),
                    help: info.help.into(),
                })
            })
            .collect()
    }
}

impl Host for AppHandle {
    fn report_failure(&self, id: &str, error: &AppError) {
        log::error!("{id} failed: {error}");
    }
}

/// Runs a command without waiting for it (menu and tray items).
pub fn spawn(app: &AppHandle, id: &str) {
    let app = app.clone();
    let id = id.to_string();
    tauri::async_runtime::spawn(async move {
        let commands = app.state::<Commands>();
        commands.run(app.clone(), &id).await;
    });
}

/// The page runs commands by id; a failure comes back as the error.
#[tauri::command]
pub async fn run_command(app: AppHandle, id: String) -> AppResult<()> {
    let work = app.state::<Commands>().start(app.clone(), &id)?;
    work.await
}
