//! wp-env has no status command, but it names its Docker Compose project after the md5 of the config file's
//! path, so Docker's running services tell whether a project's site runs and on which port.

use std::path::Path;

use super::docker::{ComposeService, published_port};
use super::{Connection, Described, ServerState};

pub const CONFIG_FILE: &str = ".wp-env.json";
pub const OVERRIDE_FILE: &str = ".wp-env.override.json";
/// The Compose service of the development site (besides tests-wordpress, mysql, …).
const SITE_SERVICE: &str = "wordpress";
const DATABASE_SERVICE: &str = "mysql";

/// Whether the folder is set up for wp-env.
pub fn configured(dir: &Path) -> bool {
    dir.join(CONFIG_FILE).is_file() || dir.join(OVERRIDE_FILE).is_file()
}

fn compose_project(dir: &Path) -> String {
    format!(
        "{:x}",
        md5::compute(dir.join(CONFIG_FILE).to_string_lossy().as_bytes())
    )
}

fn service<'a>(
    dir: &Path,
    services: &'a [ComposeService],
    name: &str,
) -> Option<&'a ComposeService> {
    let project = compose_project(dir);
    services
        .iter()
        .find(|s| s.project == project && s.name == name)
}

/// Running with the site's URL while its Compose service runs, else stopped.
pub fn described(dir: &Path, services: &[ComposeService]) -> Described {
    match service(dir, services, SITE_SERVICE) {
        Some(site) => Described {
            state: ServerState::Running,
            url: published_port(&site.ports, 80).map(|port| format!("http://localhost:{port}")),
            detail: None,
        },
        None => Described {
            state: ServerState::Stopped,
            url: None,
            detail: None,
        },
    }
}

/// The development site's database; wp-env's credentials are fixed.
pub fn connection(dir: &Path, services: &[ComposeService]) -> Result<Connection, String> {
    let port = service(dir, services, DATABASE_SERVICE)
        .and_then(|s| published_port(&s.ports, 3306))
        .ok_or("wp-env’s database isn’t running or has no published port.")?;
    Ok(Connection {
        host: "127.0.0.1".into(),
        port,
        user: "root".into(),
        password: "password".into(),
        database: "wordpress".into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// Docker's Compose project for the folder, computed from wp-env's rule: the md5 of the config file's path.
    fn project_of(dir: &Path) -> String {
        format!(
            "{:x}",
            md5::compute(dir.join(".wp-env.json").to_string_lossy().as_bytes())
        )
    }

    fn service_of(dir: &Path, name: &str, ports: &str) -> ComposeService {
        ComposeService {
            project: project_of(dir),
            name: name.into(),
            ports: ports.into(),
        }
    }

    #[test]
    fn a_folder_is_configured_by_either_wp_env_file() {
        for (files, expected) in [
            (&[] as &[&str], false),
            (&[".wp-env.json"][..], true),
            (&[".wp-env.override.json"][..], true),
            (&[".wp-env.json", ".wp-env.override.json"][..], true),
            (&["package.json"][..], false),
        ] {
            let dir = tempfile::tempdir().unwrap();
            for file in files {
                fs::write(dir.path().join(file), "{}").unwrap();
            }

            assert_eq!(configured(dir.path()), expected, "{files:?}");
        }
    }

    #[test]
    fn a_folder_named_like_the_config_file_is_not_configured() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join(".wp-env.json")).unwrap();

        assert!(!configured(dir.path()));
    }

    #[test]
    fn a_running_site_is_running_with_its_published_url() {
        let dir = Path::new("/demo/shop");
        let services = [service_of(
            dir,
            "wordpress",
            "0.0.0.0:8888->80/tcp, :::8888->80/tcp",
        )];

        assert_eq!(
            described(dir, &services),
            Described {
                state: ServerState::Running,
                url: Some("http://localhost:8888".into()),
                detail: None,
            }
        );
    }

    #[test]
    fn a_running_site_without_a_published_port_has_no_url() {
        let dir = Path::new("/demo/shop");
        let services = [service_of(dir, "wordpress", "80/tcp")];

        assert_eq!(
            described(dir, &services),
            Described {
                state: ServerState::Running,
                url: None,
                detail: None,
            }
        );
    }

    #[test]
    fn a_site_without_its_service_is_stopped() {
        let dir = Path::new("/demo/shop");
        let other = Path::new("/demo/blog");
        let stopped = Described {
            state: ServerState::Stopped,
            url: None,
            detail: None,
        };

        assert_eq!(described(dir, &[]), stopped);
        assert_eq!(
            described(
                dir,
                &[service_of(other, "wordpress", "0.0.0.0:8889->80/tcp")]
            ),
            stopped
        );
        assert_eq!(
            described(
                dir,
                &[
                    service_of(dir, "tests-wordpress", "0.0.0.0:8889->80/tcp"),
                    service_of(dir, "mysql", "0.0.0.0:49153->3306/tcp"),
                ]
            ),
            stopped
        );
    }

    #[test]
    fn the_site_is_found_among_other_projects_services() {
        let dir = Path::new("/demo/shop");
        let other = Path::new("/demo/blog");
        let services = [
            service_of(other, "wordpress", "0.0.0.0:8889->80/tcp"),
            service_of(dir, "wordpress", "0.0.0.0:8888->80/tcp"),
        ];

        assert_eq!(
            described(dir, &services).url.as_deref(),
            Some("http://localhost:8888")
        );
    }

    #[test]
    fn the_connection_uses_wp_envs_fixed_credentials_and_the_published_mysql_port() {
        let dir = Path::new("/demo/shop");
        let other = Path::new("/demo/blog");
        let services = [
            service_of(other, "mysql", "0.0.0.0:49200->3306/tcp"),
            service_of(dir, "tests-mysql", "0.0.0.0:49154->3306/tcp"),
            service_of(dir, "mysql", "0.0.0.0:49153->3306/tcp, :::49153->3306/tcp"),
        ];

        assert_eq!(
            connection(dir, &services),
            Ok(Connection {
                host: "127.0.0.1".into(),
                port: 49153,
                user: "root".into(),
                password: "password".into(),
                database: "wordpress".into(),
            })
        );
    }

    #[test]
    fn a_connection_needs_the_projects_own_published_database() {
        let dir = Path::new("/demo/shop");
        let other = Path::new("/demo/blog");
        let message = "wp-env’s database isn’t running or has no published port.";

        for services in [
            vec![],
            vec![service_of(other, "mysql", "0.0.0.0:49200->3306/tcp")],
            vec![service_of(dir, "mysql", "3306/tcp")],
            vec![service_of(dir, "wordpress", "0.0.0.0:8888->80/tcp")],
        ] {
            assert_eq!(
                connection(dir, &services),
                Err(message.to_string()),
                "{services:?}"
            );
        }
    }
}
