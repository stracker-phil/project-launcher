//! Docker: the app that runs it (a manager preset or the owner's own commands) and the Compose services it runs.

use crate::store::{DockerManager, Settings};

/// How to start and stop Docker.
#[derive(Debug, Clone, PartialEq)]
pub struct Manager {
    pub label: String,
    pub start: String,
    pub stop: String,
}

/// The name of the app that runs Docker.
pub fn manager_label(manager: DockerManager) -> &'static str {
    match manager {
        DockerManager::DockerDesktop => "Docker Desktop",
        DockerManager::OrbStack => "OrbStack",
        DockerManager::Colima => "Colima",
        DockerManager::Custom => "Docker",
    }
}

/// The settings' manager; a custom one runs the owner's commands.
pub fn manager(settings: &Settings) -> Manager {
    let label = manager_label(settings.docker_manager).to_string();
    let preset = |start: &str, stop: &str| Manager {
        label: label.clone(),
        start: start.into(),
        stop: stop.into(),
    };
    match settings.docker_manager {
        DockerManager::DockerDesktop => preset(
            "open -a Docker",
            // `docker desktop stop` needs Docker Desktop 4.37+; older versions are asked to quit.
            r#"docker desktop stop || osascript -e 'quit app "Docker"'"#,
        ),
        DockerManager::OrbStack => preset("orb start", "orb stop"),
        DockerManager::Colima => preset("colima start", "colima stop"),
        DockerManager::Custom => Manager {
            label,
            start: settings.docker_start_command.trim().into(),
            stop: settings.docker_stop_command.trim().into(),
        },
    }
}

/// A running container of a Docker Compose project.
#[derive(Debug, Clone, PartialEq)]
pub struct ComposeService {
    pub project: String,
    pub name: String,
    /// As Docker prints them: `0.0.0.0:8888->80/tcp, …`.
    pub ports: String,
}

/// `docker ps` with this format lists what `compose_services` reads.
pub const PS_FORMAT: &str = "{{.Label \"com.docker.compose.project\"}}\t{{.Label \"com.docker.compose.service\"}}\t{{.Ports}}";

/// The Compose services in `docker ps --format PS_FORMAT` output; containers outside Compose are left out.
pub fn compose_services(output: &str) -> Vec<ComposeService> {
    output
        .lines()
        .filter_map(|line| {
            let mut parts = line.split('\t');
            let project = parts.next()?.trim();
            let name = parts.next()?.trim();
            let ports = parts.next().unwrap_or_default().trim();
            (!project.is_empty() && !name.is_empty()).then(|| ComposeService {
                project: project.into(),
                name: name.into(),
                ports: ports.into(),
            })
        })
        .collect()
}

/// The host port published for `container_port/tcp`, from Docker's ports text.
pub fn published_port(ports: &str, container_port: u16) -> Option<u16> {
    let suffix = format!("->{container_port}/tcp");
    ports.split(',').find_map(|mapping| {
        let mapping = mapping.trim();
        let host = mapping.strip_suffix(&suffix)?;
        host.rsplit(':').next()?.parse().ok()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings(manager: DockerManager, start: &str, stop: &str) -> Settings {
        Settings {
            docker_manager: manager,
            docker_start_command: start.into(),
            docker_stop_command: stop.into(),
            ..Settings::default()
        }
    }

    #[test]
    fn a_manager_label_names_the_app_that_runs_docker() {
        for (docker_manager, label) in [
            (DockerManager::DockerDesktop, "Docker Desktop"),
            (DockerManager::OrbStack, "OrbStack"),
            (DockerManager::Colima, "Colima"),
            (DockerManager::Custom, "Docker"),
        ] {
            assert_eq!(manager_label(docker_manager), label, "{docker_manager:?}");
        }
    }

    #[test]
    fn each_preset_manager_has_its_label_and_commands() {
        for (docker_manager, label, start, stop) in [
            (
                DockerManager::DockerDesktop,
                "Docker Desktop",
                "open -a Docker",
                r#"docker desktop stop || osascript -e 'quit app "Docker"'"#,
            ),
            (DockerManager::OrbStack, "OrbStack", "orb start", "orb stop"),
            (
                DockerManager::Colima,
                "Colima",
                "colima start",
                "colima stop",
            ),
        ] {
            assert_eq!(
                manager(&settings(docker_manager, "ignored up", "ignored down")),
                Manager {
                    label: label.into(),
                    start: start.into(),
                    stop: stop.into(),
                },
                "{label}"
            );
        }
    }

    #[test]
    fn a_custom_manager_runs_the_owners_trimmed_commands() {
        assert_eq!(
            manager(&settings(
                DockerManager::Custom,
                "  demo-docker up \n",
                "\tdemo-docker down  "
            )),
            Manager {
                label: "Docker".into(),
                start: "demo-docker up".into(),
                stop: "demo-docker down".into(),
            }
        );
    }

    #[test]
    fn a_custom_manager_without_commands_has_empty_ones() {
        let custom = manager(&settings(DockerManager::Custom, "   ", ""));

        assert_eq!((custom.start.as_str(), custom.stop.as_str()), ("", ""));
    }

    #[test]
    fn compose_services_are_read_from_tab_separated_lines() {
        let output = "demo-site\tweb\t0.0.0.0:8888->80/tcp, :::8888->80/tcp\n\
                      demo-site\tdb\t127.0.0.1:32768->3306/tcp\n";

        assert_eq!(
            compose_services(output),
            [
                ComposeService {
                    project: "demo-site".into(),
                    name: "web".into(),
                    ports: "0.0.0.0:8888->80/tcp, :::8888->80/tcp".into(),
                },
                ComposeService {
                    project: "demo-site".into(),
                    name: "db".into(),
                    ports: "127.0.0.1:32768->3306/tcp".into(),
                },
            ]
        );
    }

    #[test]
    fn containers_outside_compose_are_left_out() {
        let output = "\t\t0.0.0.0:5432->5432/tcp\n\
                      not tab separated\n\
                      \n\
                      demo-site\t\t80/tcp\n\
                      \tweb\t80/tcp\n\
                      demo-site\tweb\t80/tcp\n";

        assert_eq!(
            compose_services(output),
            [ComposeService {
                project: "demo-site".into(),
                name: "web".into(),
                ports: "80/tcp".into(),
            }]
        );
    }

    #[test]
    fn a_service_without_a_ports_column_has_empty_ports() {
        for output in ["demo-site\tworker", "demo-site\tworker\t"] {
            assert_eq!(
                compose_services(output),
                [ComposeService {
                    project: "demo-site".into(),
                    name: "worker".into(),
                    ports: String::new(),
                }],
                "{output:?}"
            );
        }
    }

    #[test]
    fn no_output_has_no_services() {
        assert_eq!(compose_services(""), Vec::<ComposeService>::new());
    }

    #[test]
    fn the_published_port_is_the_host_port_of_the_container_port() {
        for (ports, container, expected) in [
            ("0.0.0.0:8888->80/tcp", 80, Some(8888)),
            ("0.0.0.0:8888->80/tcp, :::8888->80/tcp", 80, Some(8888)),
            (":::8889->80/tcp", 80, Some(8889)),
            ("127.0.0.1:32768->3306/tcp", 3306, Some(32768)),
            (
                "0.0.0.0:8080->8080/tcp, 0.0.0.0:3000->3000/tcp, 0.0.0.0:9000->80/tcp",
                80,
                Some(9000),
            ),
            (
                "0.0.0.0:8080->8080/tcp, 0.0.0.0:3000->3000/tcp, 0.0.0.0:9000->80/tcp",
                3000,
                Some(3000),
            ),
        ] {
            assert_eq!(
                published_port(ports, container),
                expected,
                "{ports} / {container}"
            );
        }
    }

    #[test]
    fn a_container_port_that_is_not_published_has_no_host_port() {
        for (ports, container) in [
            ("", 80),
            ("80/tcp", 80),
            ("0.0.0.0:8888->80/tcp", 443),
            ("0.0.0.0:8080->8080/tcp", 80),
            ("0.0.0.0:53->53/udp", 53),
            ("0.0.0.0:abc->80/tcp", 80),
        ] {
            assert_eq!(
                published_port(ports, container),
                None,
                "{ports} / {container}"
            );
        }
    }
}
