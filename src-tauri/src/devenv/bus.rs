//! The `dev:` commands: a project's Docker and dev server from the palette, the tray or `launcher command`, and
//! the ones that apply right now as the palette lists them.

use tauri::{AppHandle, Manager};

use super::commands::{act_in_background, status_of};
use super::{DevAction, DevStatus, ServerState, ServerStatus};
use crate::bus::{Command, CommandBus};
use crate::error::{AppError, AppResult};
use crate::menu::palette::MenuCommand;
use crate::state::AppState;

const PREFIX: &str = "dev:";
const OPEN_SITE: &str = "open-site";
/// The palette's section for these, before it puts them under the project's name.
const MENU: &str = "Dev Environment";

const ACTIONS: [(&str, DevAction); 6] = [
    ("docker-start", DevAction::DockerStart),
    ("docker-stop", DevAction::DockerStop),
    ("docker-restart", DevAction::DockerRestart),
    ("server-start", DevAction::ServerStart),
    ("server-stop", DevAction::ServerStop),
    ("server-restart", DevAction::ServerRestart),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Step {
    Act(DevAction),
    OpenSite,
}

fn step_name(step: Step) -> &'static str {
    match step {
        Step::OpenSite => OPEN_SITE,
        Step::Act(action) => ACTIONS
            .iter()
            .find(|(_, a)| *a == action)
            .map(|(name, _)| *name)
            .unwrap_or_default(),
    }
}

/// `dev:<step>:<project id>`.
fn command(step: Step, id: &str) -> String {
    format!("{PREFIX}{}:{id}", step_name(step))
}

/// `<step>:<project id>`, as `command` writes it after the prefix.
fn parse(argument: &str) -> Option<(Step, &str)> {
    let (name, id) = argument.split_once(':')?;
    if id.is_empty() {
        return None;
    }
    let step = if name == OPEN_SITE {
        Step::OpenSite
    } else {
        Step::Act(ACTIONS.iter().find(|(n, _)| *n == name)?.1)
    };
    Some((step, id))
}

/// What the palette offers for the project's dev environment now: a start for what is stopped, a restart and a
/// stop for what runs, the site of a running server; nothing for a part that is busy. A server's start shows
/// while Docker is stopped too, as starting the server starts Docker first.
pub fn palette_items(id: &str, status: &DevStatus) -> Vec<MenuCommand> {
    use DevAction::*;
    let mut steps: Vec<(String, Step)> = Vec::new();
    let docker_busy = status.docker.as_ref().is_some_and(|d| d.activity.is_some());
    if let Some(docker) = status.docker.as_ref().filter(|_| !docker_busy) {
        if docker.running {
            steps.push(("Restart Docker".into(), Step::Act(DockerRestart)));
            steps.push(("Stop Docker".into(), Step::Act(DockerStop)));
        } else {
            steps.push(("Start Docker".into(), Step::Act(DockerStart)));
        }
    }
    let idle = |s: &&ServerStatus| s.activity.is_none() && s.state != ServerState::NotConfigured;
    if let Some(server) = status.server.as_ref().filter(idle).filter(|_| !docker_busy) {
        let label = &server.label;
        if server.state == ServerState::Running {
            steps.push((format!("Restart {label}"), Step::Act(ServerRestart)));
            steps.push((format!("Stop {label}"), Step::Act(ServerStop)));
            if server.url.is_some() {
                steps.push(("Open Site".into(), Step::OpenSite));
            }
        } else {
            steps.push((format!("Start {label}"), Step::Act(ServerStart)));
        }
    }
    steps
        .into_iter()
        .map(|(label, step)| MenuCommand {
            menu: MENU.into(),
            label,
            command: command(step, id),
            key: None,
        })
        .collect()
}

/// The running server's site in the browser, as the detail's globe button opens it.
async fn open_site(app: AppHandle, id: String) -> AppResult<()> {
    let status = status_of(app.clone(), id).await?;
    let url = status
        .server
        .filter(|server| server.state == ServerState::Running)
        .and_then(|server| server.url)
        .ok_or_else(|| AppError::InvalidInput("The dev server isn't running.".into()))?;
    crate::commands::open_url(app, url)
}

pub fn register(bus: &mut CommandBus<AppHandle>) {
    bus.add_family(
        Command::new(PREFIX, "Dev Environment")
            .help("Start, stop or restart a project's Docker or dev server, or open its site")
            .hidden()
            .family("<step>:<id>")
            .run(|app: AppHandle, argument: String| async move {
                let (step, id) = parse(&argument).ok_or_else(|| {
                    AppError::InvalidInput(format!("Not a dev environment step: {argument}"))
                })?;
                app.state::<AppState>().data.project(id)?;
                match step {
                    Step::Act(action) => {
                        act_in_background(&app, id.into(), action);
                        Ok(())
                    }
                    Step::OpenSite => open_site(app, id.into()).await,
                }
            }),
    );
}

/// The `dev:` commands that apply to the project now, for the palette.
#[tauri::command]
pub async fn dev_commands(app: AppHandle, id: String) -> AppResult<Vec<MenuCommand>> {
    let status = status_of(app, id.clone()).await?;
    Ok(palette_items(&id, &status))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::devenv::Server;
    use crate::devenv::{Activity, DockerStatus};

    fn docker(running: bool, activity: Option<Activity>) -> Option<DockerStatus> {
        Some(DockerStatus {
            running,
            activity,
            manager: "Docker Desktop".into(),
        })
    }

    fn server(
        kind: Server,
        state: ServerState,
        url: Option<&str>,
        activity: Option<Activity>,
    ) -> Option<ServerStatus> {
        Some(ServerStatus {
            server: kind,
            label: kind.label().into(),
            state,
            url: url.map(String::from),
            detail: None,
            activity,
        })
    }

    fn status(docker: Option<DockerStatus>, server: Option<ServerStatus>) -> DevStatus {
        DevStatus {
            docker,
            server,
            database: None,
        }
    }

    fn labels(status: &DevStatus) -> Vec<String> {
        palette_items("demo-site", status)
            .into_iter()
            .map(|item| item.label)
            .collect()
    }

    #[test]
    fn stopped_docker_offers_only_start_docker() {
        assert_eq!(labels(&status(docker(false, None), None)), ["Start Docker"]);
    }

    #[test]
    fn running_docker_offers_restart_and_stop() {
        assert_eq!(
            labels(&status(docker(true, None), None)),
            ["Restart Docker", "Stop Docker"]
        );
    }

    #[test]
    fn a_stopped_server_offers_its_start_even_while_docker_is_down() {
        for (kind, expected) in [
            (Server::Ddev, "Start DDEV"),
            (Server::WpEnv, "Start wp-env"),
        ] {
            for running in [true, false] {
                let status = status(
                    docker(running, None),
                    server(kind, ServerState::Stopped, None, None),
                );

                assert_eq!(
                    labels(&status).last().map(String::as_str),
                    Some(expected),
                    "{expected}, docker running: {running}"
                );
            }
        }
    }

    #[test]
    fn a_running_server_offers_restart_stop_and_its_site() {
        let status = status(
            docker(true, None),
            server(
                Server::Ddev,
                ServerState::Running,
                Some("https://demo-site.ddev.site"),
                None,
            ),
        );

        assert_eq!(
            labels(&status),
            [
                "Restart Docker",
                "Stop Docker",
                "Restart DDEV",
                "Stop DDEV",
                "Open Site"
            ]
        );
    }

    #[test]
    fn a_running_server_without_a_url_has_no_open_site() {
        let status = status(
            None,
            server(Server::WpEnv, ServerState::Running, None, None),
        );

        assert_eq!(labels(&status), ["Restart wp-env", "Stop wp-env"]);
    }

    #[test]
    fn a_busy_server_gets_no_rows_but_docker_keeps_its_own() {
        let status = status(
            docker(true, None),
            server(
                Server::Ddev,
                ServerState::Running,
                Some("https://demo-site.ddev.site"),
                Some(Activity::Restarting),
            ),
        );

        assert_eq!(labels(&status), ["Restart Docker", "Stop Docker"]);
    }

    #[test]
    fn a_busy_docker_gets_no_rows_and_no_server_rows() {
        let status = status(
            docker(false, Some(Activity::Starting)),
            server(Server::Ddev, ServerState::Stopped, None, None),
        );

        assert_eq!(labels(&status), Vec::<String>::new());
    }

    #[test]
    fn a_server_that_is_not_configured_gets_no_rows() {
        let status = status(
            docker(true, None),
            server(Server::Ddev, ServerState::NotConfigured, None, None),
        );

        assert_eq!(labels(&status), ["Restart Docker", "Stop Docker"]);
    }

    #[test]
    fn a_project_without_docker_or_server_has_no_rows() {
        assert_eq!(labels(&DevStatus::default()), Vec::<String>::new());
    }

    #[test]
    fn rows_carry_the_projects_dev_command_in_the_dev_environment_section() {
        let status = status(
            docker(true, None),
            server(
                Server::Ddev,
                ServerState::Running,
                Some("https://demo-site.ddev.site"),
                None,
            ),
        );

        let items = palette_items("demo-site", &status);

        assert_eq!(
            items,
            [
                ("Restart Docker", "dev:docker-restart:demo-site"),
                ("Stop Docker", "dev:docker-stop:demo-site"),
                ("Restart DDEV", "dev:server-restart:demo-site"),
                ("Stop DDEV", "dev:server-stop:demo-site"),
                ("Open Site", "dev:open-site:demo-site"),
            ]
            .map(|(label, command)| MenuCommand {
                menu: "Dev Environment".into(),
                label: label.into(),
                command: command.into(),
                key: None,
            })
        );
    }

    #[test]
    fn start_rows_use_the_start_step_ids() {
        let status = status(
            docker(false, None),
            server(Server::WpEnv, ServerState::Stopped, None, None),
        );

        let commands: Vec<String> = palette_items("demo-shop", &status)
            .into_iter()
            .map(|item| item.command)
            .collect();

        assert_eq!(
            commands,
            ["dev:docker-start:demo-shop", "dev:server-start:demo-shop"]
        );
    }

    #[test]
    fn every_step_round_trips_through_its_command_id() {
        let steps = ACTIONS
            .iter()
            .map(|(_, action)| Step::Act(*action))
            .chain([Step::OpenSite]);
        for step in steps {
            let id = command(step, "demo-site");
            let argument = id.strip_prefix(PREFIX).unwrap();

            assert_eq!(parse(argument), Some((step, "demo-site")), "{id}");
        }
    }

    #[test]
    fn a_project_id_with_a_colon_stays_whole() {
        assert_eq!(
            parse("server-stop:demo:site"),
            Some((Step::Act(DevAction::ServerStop), "demo:site"))
        );
    }

    #[test]
    fn a_missing_step_an_empty_id_or_an_unknown_step_is_refused() {
        for argument in [
            "",
            "docker-start",
            ":demo-site",
            "docker-start:",
            "reboot:demo-site",
        ] {
            assert_eq!(parse(argument), None, "{argument:?}");
        }
    }
}
