//! Bus commands that act on a project by id: what the tray's rows and the command line run.

use tauri::{AppHandle, Emitter, Manager};

use super::Runner;
use crate::bus::{Command, CommandBus};
use crate::data::Changed;
use crate::data::commands::announce;
use crate::error::{AppError, AppResult};
use crate::events::SELECT_PROJECT_EVENT;
use crate::library::Filter;
use crate::projects::config::{ActionList, ActionSource};
use crate::state::AppState;
use crate::window;

const SHOW: &str = "project:show:";
const LAUNCH: &str = "project:launch:";
const REVEAL: &str = "project:reveal:";

pub fn show_command(id: &str) -> String {
    format!("{SHOW}{id}")
}

pub fn launch_command(id: &str, source: ActionSource) -> String {
    format!("{LAUNCH}{id}:{}:{}", source.list.key(), source.index)
}

pub fn reveal_command(id: &str) -> String {
    format!("{REVEAL}{id}")
}

/// `<id>:<list>:<index>`, as `launch_command` writes it.
fn parse_launch(argument: &str) -> Option<(&str, ActionSource)> {
    let (rest, index) = argument.rsplit_once(':')?;
    let (id, list) = rest.rsplit_once(':')?;
    let list = [ActionList::Apps, ActionList::Scripts]
        .into_iter()
        .find(|l| l.key() == list)?;
    Some((
        id,
        ActionSource {
            list,
            index: index.parse().ok()?,
        },
    ))
}

/// Runs `work` with a runner off the async runtime, then announces the changed last-opened time.
async fn opening(
    app: AppHandle,
    work: impl FnOnce(&Runner, i64) -> AppResult<()> + Send + 'static,
) -> AppResult<()> {
    let handle = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let state = handle.state::<AppState>();
        work(&state.runner(), chrono::Utc::now().timestamp_millis())
    })
    .await
    .map_err(|e| AppError::Internal(e.to_string()))??;
    announce(
        &app,
        Changed {
            settings: false,
            projects: true,
        },
    );
    Ok(())
}

/// Shows the main window with the project selected (under Archived when it is archived).
fn show_project(app: &AppHandle, id: &str) -> AppResult<()> {
    let archived = app.state::<AppState>().data.project(id)?.meta.archived;
    let filter = if archived {
        Filter::Archived
    } else {
        Filter::All
    };
    window::show_main(app);
    let payload = serde_json::json!({ "id": id, "filter": filter });
    if let Err(e) = app.emit_to(window::MAIN, SELECT_PROJECT_EVENT, payload) {
        log::warn!("Couldn't select the project: {e}");
    }
    Ok(())
}

pub fn register(bus: &mut CommandBus<AppHandle>) {
    bus.add_family(
        Command::new(SHOW, "Show Project")
            .help("Show the Launcher window with a project selected")
            .hidden()
            .family("<id>")
            .run(|app, id| async move { show_project(&app, &id) }),
    );
    bus.add_family(
        Command::new(LAUNCH, "Launch Action")
            .help("Run one of a project's actions")
            .hidden()
            .family("<id>:<list>:<index>")
            .run(|app, argument| async move {
                let (id, source) = parse_launch(&argument)
                    .ok_or_else(|| AppError::InvalidInput(format!("Not an action: {argument}")))?;
                let id = id.to_string();
                opening(app, move |runner, now| {
                    runner.launch(&id, source, now).map(|_| ())
                })
                .await
            }),
    );
    bus.add_family(
        Command::new(REVEAL, "Reveal Project in Finder")
            .help("Show a project's folder in Finder")
            .hidden()
            .family("<id>")
            .run(|app, id| async move {
                opening(app, move |runner, now| runner.reveal(&id, now)).await
            }),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source(list: ActionList, index: usize) -> ActionSource {
        ActionSource { list, index }
    }

    #[test]
    fn the_tray_commands_are_built_from_the_project_id() {
        assert_eq!(show_command("acme-website"), "project:show:acme-website");
        assert_eq!(
            reveal_command("acme-website"),
            "project:reveal:acme-website"
        );
        assert_eq!(
            launch_command("acme-website", source(ActionList::Apps, 2)),
            "project:launch:acme-website:apps:2"
        );
        assert_eq!(
            launch_command("acme-website", source(ActionList::Scripts, 0)),
            "project:launch:acme-website:scripts:0"
        );
    }

    #[test]
    fn a_launch_command_parses_back_to_its_project_and_action() {
        for list in [ActionList::Apps, ActionList::Scripts] {
            let expected = source(list, 7);
            let command = launch_command("acme-website-2", expected);
            let argument = command.strip_prefix("project:launch:").unwrap();
            assert_eq!(parse_launch(argument), Some(("acme-website-2", expected)));
        }
    }

    #[test]
    fn a_launch_argument_names_the_project_the_list_and_the_index() {
        for (argument, expected) in [
            (
                "northwind:apps:0",
                ("northwind", source(ActionList::Apps, 0)),
            ),
            (
                "northwind:scripts:12",
                ("northwind", source(ActionList::Scripts, 12)),
            ),
        ] {
            assert_eq!(parse_launch(argument), Some(expected), "{argument}");
        }
    }

    #[test]
    fn a_project_id_with_dashes_is_kept_whole() {
        assert_eq!(
            parse_launch("acme-website-2:apps:3"),
            Some(("acme-website-2", source(ActionList::Apps, 3)))
        );
    }

    #[test]
    fn a_launch_argument_with_a_bad_list_a_bad_index_or_missing_parts_is_refused() {
        for argument in [
            "northwind:tools:0",
            "northwind:Apps:0",
            "northwind:apps:first",
            "northwind:apps:-1",
            "northwind:apps:",
            "northwind:apps",
            "northwind",
            "apps:0",
            "",
        ] {
            assert_eq!(parse_launch(argument), None, "{argument:?}");
        }
    }
}
