//! The port to Docker, DDEV and wp-env. `Real` runs them through the `System` port; debug builds use `Stand-in`
//! unless `LAUNCHER_REAL_DOCKER=1`, so the debug app never starts or stops the owner's containers (ADR 0022).

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use super::docker::{self, Manager};
use super::{Connection, Described, Failure, Server, ServerState, ddev, wpenv};
use crate::run::system::{Job, System};

const DOCKER_START_TIMEOUT: Duration = Duration::from_secs(3 * 60);
const DOCKER_STOP_TIMEOUT: Duration = Duration::from_secs(60);
/// Server commands can pull images and take minutes.
const SERVER_TIMEOUT: Duration = Duration::from_secs(10 * 60);
const DOCKER_POLL: Duration = Duration::from_secs(2);

pub trait Tools: Send + Sync {
    /// True once the Docker engine answers; an app that is still booting doesn't count.
    fn docker_running(&self) -> bool;
    /// A project's server; only asked while Docker runs.
    fn describe(&self, server: Server, dir: &Path) -> Described;
    fn connection(&self, server: Server, dir: &Path) -> Result<Connection, String>;
    /// Starts Docker and waits until it answers.
    fn start_docker(&self, manager: &Manager) -> Result<(), Failure>;
    fn stop_docker(&self, manager: &Manager) -> Result<(), Failure>;
    fn start_server(
        &self,
        server: Server,
        dir: &Path,
        env: &[(String, String)],
    ) -> Result<(), Failure>;
    fn stop_server(
        &self,
        server: Server,
        dir: &Path,
        env: &[(String, String)],
    ) -> Result<(), Failure>;
    /// Stops every DDEV project; they need a clean shutdown before Docker goes.
    fn stop_ddev_projects(&self) -> Result<(), Failure>;
    /// The folders among `servers` whose server runs, asked in one go; only asked while Docker runs.
    fn running_servers(&self, servers: &[(Server, PathBuf)]) -> Vec<PathBuf>;
}

/// The tools the app runs on.
pub fn tools(system: Arc<dyn System>) -> Box<dyn Tools> {
    #[cfg(debug_assertions)]
    if std::env::var("LAUNCHER_REAL_DOCKER").as_deref() != Ok("1") {
        return Box::new(StandIn::default());
    }
    Box::new(Real { system })
}

pub struct Real {
    pub system: Arc<dyn System>,
}

// npx prefers the project's own @wordpress/env and falls back to a global wp-env.
fn start_command(server: Server) -> &'static str {
    match server {
        Server::Ddev => "ddev start",
        Server::WpEnv => "npx wp-env start",
    }
}

fn stop_command(server: Server) -> &'static str {
    match server {
        Server::Ddev => "ddev stop",
        Server::WpEnv => "npx wp-env stop",
    }
}

impl Real {
    fn step(
        &self,
        label: &str,
        command: &str,
        dir: Option<&Path>,
        env: &[(String, String)],
        timeout: Duration,
    ) -> Result<(), Failure> {
        let output = self.system.run(
            &Job {
                command: command.into(),
                cwd: dir.map(Path::to_path_buf),
                env: env.to_vec(),
            },
            timeout,
        );
        if output.ok() {
            return Ok(());
        }
        let reason = match output.code {
            _ if output.timed_out => "Timed out.".to_string(),
            Some(code) => format!("Exit code {code}."),
            None => "It was stopped.".to_string(),
        };
        let printed = output.output.trim();
        Err(Failure::new(
            label,
            if printed.is_empty() {
                reason
            } else {
                printed.to_string()
            },
        ))
    }

    fn wait_docker(&self, running: bool, timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        while Instant::now() < deadline {
            if self.docker_running() == running {
                return true;
            }
            thread::sleep(DOCKER_POLL);
        }
        false
    }

    fn ddev(&self, dir: Option<&Path>, args: &[&str]) -> String {
        let mut all = args.to_vec();
        all.push("--json-output");
        self.system.inspect(dir, "ddev", &all).output
    }

    fn compose_services(&self) -> Vec<docker::ComposeService> {
        self.system
            .tool("docker", &["ps", "--format", docker::PS_FORMAT])
            .map(|output| docker::compose_services(&output))
            .unwrap_or_default()
    }
}

impl Tools for Real {
    fn running_servers(&self, servers: &[(Server, PathBuf)]) -> Vec<PathBuf> {
        let has = |kind: Server| servers.iter().any(|(server, _)| *server == kind);
        let ddev: Vec<PathBuf> = if has(Server::Ddev) {
            ddev::running_paths(&self.ddev(None, &["list"]))
        } else {
            Vec::new()
        };
        let compose = if has(Server::WpEnv) {
            self.compose_services()
        } else {
            Vec::new()
        };
        servers
            .iter()
            .filter(|(server, dir)| match server {
                Server::Ddev => ddev.contains(dir),
                Server::WpEnv => wpenv::described(dir, &compose).state == ServerState::Running,
            })
            .map(|(_, dir)| dir.clone())
            .collect()
    }

    fn docker_running(&self) -> bool {
        self.system
            .tool("docker", &["info", "--format", "{{.ServerVersion}}"])
            .is_some()
    }

    fn describe(&self, server: Server, dir: &Path) -> Described {
        match server {
            Server::Ddev => ddev::described(&self.ddev(Some(dir), &["describe"])),
            Server::WpEnv => wpenv::described(dir, &self.compose_services()),
        }
    }

    fn connection(&self, server: Server, dir: &Path) -> Result<Connection, String> {
        match server {
            Server::Ddev => ddev::connection(&self.ddev(Some(dir), &["describe"])),
            Server::WpEnv => wpenv::connection(dir, &self.compose_services()),
        }
    }

    fn start_docker(&self, manager: &Manager) -> Result<(), Failure> {
        let label = "Start Docker";
        if manager.start.is_empty() {
            return Err(Failure::new(
                label,
                "No start command is set in Settings → General.",
            ));
        }
        self.step(label, &manager.start, None, &[], DOCKER_START_TIMEOUT)?;
        if self.wait_docker(true, DOCKER_START_TIMEOUT) {
            Ok(())
        } else {
            Err(Failure::new(label, "Docker wasn’t ready in time."))
        }
    }

    fn stop_docker(&self, manager: &Manager) -> Result<(), Failure> {
        let label = "Stop Docker";
        if manager.stop.is_empty() {
            return Err(Failure::new(
                label,
                "No stop command is set in Settings → General.",
            ));
        }
        self.step(label, &manager.stop, None, &[], DOCKER_STOP_TIMEOUT)?;
        if self.wait_docker(false, DOCKER_STOP_TIMEOUT) {
            Ok(())
        } else {
            Err(Failure::new(label, "Docker didn’t stop in time."))
        }
    }

    fn start_server(
        &self,
        server: Server,
        dir: &Path,
        env: &[(String, String)],
    ) -> Result<(), Failure> {
        let label = format!("Start {}", server.label());
        self.step(
            &label,
            start_command(server),
            Some(dir),
            env,
            SERVER_TIMEOUT,
        )
    }

    fn stop_server(
        &self,
        server: Server,
        dir: &Path,
        env: &[(String, String)],
    ) -> Result<(), Failure> {
        let label = format!("Stop {}", server.label());
        self.step(&label, stop_command(server), Some(dir), env, SERVER_TIMEOUT)
    }

    fn stop_ddev_projects(&self) -> Result<(), Failure> {
        if ddev::running_paths(&self.ddev(None, &["list"])).is_empty() {
            return Ok(());
        }
        self.step(
            "Stop DDEV projects",
            "ddev poweroff",
            None,
            &[],
            SERVER_TIMEOUT,
        )
    }
}

/// The debug build's Docker: running, with servers that start and stop after a moment, in memory.
#[cfg(any(debug_assertions, test, feature = "test-util"))]
pub use stand_in::StandIn;

#[cfg(any(debug_assertions, test, feature = "test-util"))]
mod stand_in {
    use std::collections::HashSet;
    use std::path::{Path, PathBuf};
    use std::sync::Mutex;
    use std::thread;
    use std::time::Duration;

    use super::Tools;
    use crate::devenv::docker::Manager;
    use crate::devenv::{Connection, Described, Failure, Server, ServerState};
    use crate::lock::MutexExt;

    /// How long a step takes, so the detail shows it under way.
    const STEP: Duration = Duration::from_millis(800);

    pub struct StandIn {
        docker: Mutex<bool>,
        running: Mutex<HashSet<PathBuf>>,
        step: Duration,
    }

    impl Default for StandIn {
        fn default() -> Self {
            StandIn {
                docker: Mutex::new(true),
                running: Mutex::default(),
                step: STEP,
            }
        }
    }

    impl StandIn {
        /// Steps finish at once; for tests.
        pub fn instant() -> Self {
            StandIn {
                step: Duration::ZERO,
                ..StandIn::default()
            }
        }

        fn wait(&self) {
            thread::sleep(self.step);
        }
    }

    impl Tools for StandIn {
        fn docker_running(&self) -> bool {
            *self.docker.lock_unpoisoned()
        }

        fn describe(&self, server: Server, dir: &Path) -> Described {
            if !self.running.lock_unpoisoned().contains(dir) {
                return Described {
                    state: ServerState::Stopped,
                    url: None,
                    detail: None,
                };
            }
            let folder = dir
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            Described {
                state: ServerState::Running,
                url: Some(match server {
                    Server::Ddev => format!("https://{folder}.ddev.site"),
                    Server::WpEnv => "http://localhost:8888".into(),
                }),
                detail: None,
            }
        }

        fn running_servers(&self, servers: &[(Server, PathBuf)]) -> Vec<PathBuf> {
            let running = self.running.lock_unpoisoned();
            servers
                .iter()
                .filter(|(_, dir)| running.contains(dir))
                .map(|(_, dir)| dir.clone())
                .collect()
        }

        fn connection(&self, _: Server, _: &Path) -> Result<Connection, String> {
            Ok(Connection {
                host: "127.0.0.1".into(),
                port: 32768,
                user: "db".into(),
                password: "db".into(),
                database: "db".into(),
            })
        }

        fn start_docker(&self, _: &Manager) -> Result<(), Failure> {
            self.wait();
            *self.docker.lock_unpoisoned() = true;
            Ok(())
        }

        fn stop_docker(&self, _: &Manager) -> Result<(), Failure> {
            self.wait();
            *self.docker.lock_unpoisoned() = false;
            Ok(())
        }

        fn start_server(
            &self,
            _: Server,
            dir: &Path,
            _: &[(String, String)],
        ) -> Result<(), Failure> {
            self.wait();
            self.running.lock_unpoisoned().insert(dir.to_path_buf());
            Ok(())
        }

        fn stop_server(
            &self,
            _: Server,
            dir: &Path,
            _: &[(String, String)],
        ) -> Result<(), Failure> {
            self.wait();
            self.running.lock_unpoisoned().remove(dir);
            Ok(())
        }

        fn stop_ddev_projects(&self) -> Result<(), Failure> {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_stand_in_reports_only_the_servers_that_were_started() {
        let tools = StandIn::instant();
        let first = PathBuf::from("/work/first-site");
        let second = PathBuf::from("/work/second-site");
        let servers = [
            (Server::Ddev, first.clone()),
            (Server::WpEnv, second.clone()),
        ];
        assert_eq!(tools.running_servers(&servers), Vec::<PathBuf>::new());

        tools.start_server(Server::WpEnv, &second, &[]).unwrap();

        assert_eq!(tools.running_servers(&servers), [second.clone()]);

        tools.start_server(Server::Ddev, &first, &[]).unwrap();
        tools.stop_server(Server::WpEnv, &second, &[]).unwrap();

        assert_eq!(tools.running_servers(&servers), [first]);
    }

    #[test]
    fn the_stand_in_ignores_started_folders_it_is_not_asked_about() {
        let tools = StandIn::instant();
        let started = PathBuf::from("/work/first-site");
        tools.start_server(Server::Ddev, &started, &[]).unwrap();

        let asked = [(Server::Ddev, PathBuf::from("/work/other-site"))];

        assert_eq!(tools.running_servers(&asked), Vec::<PathBuf>::new());
    }
}
