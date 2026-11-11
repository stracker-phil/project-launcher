//! The dev environment: Docker and a project's DDEV or wp-env server (their status, starting and stopping them,
//! one at a time per project and one Docker start shared by everyone waiting for it) and the project's database
//! in DataBee. The tools themselves are behind the `Tools` port (`tools.rs`): the real ones, or the debug build's
//! stand-in unless `LAUNCHER_REAL_DOCKER=1` (ADR 0022).

pub mod commands;
pub mod database;
mod ddev;
mod docker;
mod model;
pub mod tools;
mod wpenv;

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde_json::Value;

use crate::lock::MutexExt;
use crate::projects::config::DevMode;
use crate::store::Settings;
pub use database::DatabaseInfo;
pub use docker::{Manager, manager, manager_label};
pub use model::*;
use tools::Tools;

#[derive(Default)]
struct Activities {
    docker: Option<Activity>,
    servers: HashMap<PathBuf, Activity>,
}

/// Runs the dev environment's steps and remembers what is under way; `changed` hears every change of that.
pub struct DevEnv {
    tools: Box<dyn Tools>,
    activity: Mutex<Activities>,
    /// Projects whose environment was prepared, or controlled by hand, this session.
    prepared: Mutex<HashSet<String>>,
    /// Held while Docker starts, so everyone who needs it waits for the one start.
    docker_start: Mutex<()>,
    changed: Box<dyn Fn() + Send + Sync>,
}

impl DevEnv {
    pub fn new(tools: Box<dyn Tools>, changed: Box<dyn Fn() + Send + Sync>) -> Self {
        DevEnv {
            tools,
            activity: Mutex::default(),
            prepared: Mutex::default(),
            docker_start: Mutex::new(()),
            changed,
        }
    }

    fn set_docker(&self, activity: Option<Activity>) {
        self.activity.lock_unpoisoned().docker = activity;
        (self.changed)();
    }

    fn set_server(&self, dir: &Path, activity: Option<Activity>) {
        {
            let mut activities = self.activity.lock_unpoisoned();
            match activity {
                Some(activity) => activities.servers.insert(dir.to_path_buf(), activity),
                None => activities.servers.remove(dir),
            };
        }
        (self.changed)();
    }

    pub fn status(&self, target: &Target, settings: &Settings) -> DevStatus {
        let database = target.database_info();
        if !target.uses_docker() {
            return DevStatus {
                database,
                ..DevStatus::default()
            };
        }
        let running = self.tools.docker_running();
        let (docker_activity, server_activity) = {
            let activities = self.activity.lock_unpoisoned();
            (
                activities.docker,
                activities.servers.get(&target.dir).copied(),
            )
        };
        let server = target.server().map(|server| {
            let described = match server.missing_setup(&target.dir) {
                Some(missing) => Described {
                    state: ServerState::NotConfigured,
                    url: None,
                    detail: Some(missing),
                },
                None if !running => Described {
                    state: ServerState::Stopped,
                    url: None,
                    detail: None,
                },
                None => self.tools.describe(server, &target.dir),
            };
            ServerStatus {
                server,
                label: server.label().into(),
                state: described.state,
                url: described.url,
                detail: described.detail,
                activity: server_activity,
            }
        });
        DevStatus {
            docker: Some(DockerStatus {
                running,
                activity: docker_activity,
                manager: manager(settings).label,
            }),
            server,
            database,
        }
    }

    /// The ids of the targets whose server runs; none while Docker doesn't.
    pub fn running(&self, targets: &[Target]) -> Vec<String> {
        let servers: Vec<(Server, PathBuf)> = targets
            .iter()
            .filter_map(|t| Some((t.server()?, t.dir.clone())))
            .collect();
        if servers.is_empty() || !self.tools.docker_running() {
            return Vec::new();
        }
        let running = self.tools.running_servers(&servers);
        targets
            .iter()
            .filter(|t| running.contains(&t.dir))
            .map(|t| t.id.clone())
            .collect()
    }

    /// Starts Docker unless it runs; callers at the same time share the one start.
    fn ensure_docker(&self, settings: &Settings) -> Result<(), Failure> {
        let _starting = self.docker_start.lock_unpoisoned();
        if self.tools.docker_running() {
            return Ok(());
        }
        self.set_docker(Some(Activity::Starting));
        let result = self.tools.start_docker(&manager(settings));
        self.set_docker(None);
        result
    }

    /// Runs `step` with `activity` on Docker unless something already runs there.
    fn with_docker(
        &self,
        activity: Activity,
        step: impl FnOnce() -> Result<(), Failure>,
    ) -> Result<(), Failure> {
        if self.activity.lock_unpoisoned().docker.is_some() {
            return Ok(());
        }
        self.set_docker(Some(activity));
        let result = step();
        self.set_docker(None);
        result
    }

    /// Runs `step` with `activity` on the project's server unless something already runs there.
    fn with_server(
        &self,
        dir: &Path,
        activity: Activity,
        step: impl FnOnce() -> Result<(), Failure>,
    ) -> Result<(), Failure> {
        if self.activity.lock_unpoisoned().servers.contains_key(dir) {
            return Ok(());
        }
        self.set_server(dir, Some(activity));
        let result = step();
        self.set_server(dir, None);
        result
    }

    /// Stops the DDEV projects (they need a clean shutdown), then Docker.
    fn stop_docker(&self, settings: &Settings) -> Result<(), Failure> {
        self.tools.stop_ddev_projects()?;
        self.tools.stop_docker(&manager(settings))
    }

    /// Returns whether it wasn't prepared yet.
    fn mark_prepared(&self, id: &str) -> bool {
        self.prepared.lock_unpoisoned().insert(id.to_string())
    }

    fn server_of(target: &Target) -> Result<Server, Failure> {
        let server = target.server().ok_or_else(|| {
            Failure::new(
                "Dev environment",
                "This project has no dev server (DDEV, wp-env) set up.",
            )
        })?;
        set_up(server, &target.dir)?;
        Ok(server)
    }

    fn start_server(
        &self,
        server: Server,
        target: &Target,
        settings: &Settings,
    ) -> Result<(), Failure> {
        self.ensure_docker(settings)?;
        self.tools.start_server(server, &target.dir, &target.env)
    }

    /// Runs an action from the detail or a menu; the project counts as prepared from then on.
    pub fn act(
        &self,
        target: &Target,
        action: DevAction,
        settings: &Settings,
    ) -> Result<(), Failure> {
        self.mark_prepared(&target.id);
        let dir = &target.dir;
        match action {
            DevAction::DockerStart => self.ensure_docker(settings),
            DevAction::DockerStop => {
                self.with_docker(Activity::Stopping, || self.stop_docker(settings))
            }
            DevAction::DockerRestart => self.with_docker(Activity::Restarting, || {
                self.stop_docker(settings)?;
                self.tools.start_docker(&manager(settings))
            }),
            DevAction::ServerStart => {
                let server = Self::server_of(target)?;
                self.with_server(dir, Activity::Starting, || {
                    self.start_server(server, target, settings)
                })
            }
            DevAction::ServerStop => {
                let server = Self::server_of(target)?;
                self.with_server(dir, Activity::Stopping, || {
                    self.tools.stop_server(server, dir, &target.env)
                })
            }
            DevAction::ServerRestart => {
                let server = Self::server_of(target)?;
                self.with_server(dir, Activity::Restarting, || {
                    self.ensure_docker(settings)?;
                    self.tools.stop_server(server, dir, &target.env)?;
                    self.tools.start_server(server, dir, &target.env)
                })
            }
        }
    }

    /// When something of the project opens: starts Docker, then its server, once per session, so an environment
    /// stopped on purpose stays stopped.
    pub fn prepare(&self, target: &Target, settings: &Settings) -> Result<(), Failure> {
        if target.mode == DevMode::None || !self.mark_prepared(&target.id) {
            return Ok(());
        }
        match target
            .server()
            .filter(|s| s.missing_setup(&target.dir).is_none())
        {
            None => self.ensure_docker(settings),
            Some(server) if self.server_running(server, &target.dir) => Ok(()),
            Some(server) => self.with_server(&target.dir, Activity::Starting, || {
                self.start_server(server, target, settings)
            }),
        }
    }

    fn server_running(&self, server: Server, dir: &Path) -> bool {
        self.tools.docker_running()
            && self.tools.describe(server, dir).state == ServerState::Running
    }

    /// Whether the database's server runs (a URL's database always counts as reachable).
    pub fn database_ready(&self, target: &Target) -> bool {
        match target.database_info().map(|db| db.source) {
            Some(database::DatabaseSource::Ddev) => self.server_running(Server::Ddev, &target.dir),
            Some(database::DatabaseSource::WpEnv) => {
                self.server_running(Server::WpEnv, &target.dir)
            }
            _ => true,
        }
    }

    /// Starts the database's server unless it runs; refused while another step runs there.
    fn ensure_database_server(
        &self,
        server: Server,
        target: &Target,
        settings: &Settings,
    ) -> Result<(), Failure> {
        if self.server_running(server, &target.dir) {
            return Ok(());
        }
        self.mark_prepared(&target.id);
        if self
            .activity
            .lock_unpoisoned()
            .servers
            .contains_key(&target.dir)
        {
            return Err(Failure::new(
                format!("Start {}", server.label()),
                format!("{} is busy. Try again once it runs.", server.label()),
            ));
        }
        self.with_server(&target.dir, Activity::Starting, || {
            self.start_server(server, target, settings)
        })
    }

    /// DataBee's launch for the project's database; starts a stopped server first.
    pub fn database_launch(
        &self,
        target: &Target,
        settings: &Settings,
        home: Option<&Path>,
    ) -> Result<database::Launch, Failure> {
        let label = "Open Database";
        let info = target
            .database_info()
            .ok_or_else(|| Failure::new(label, "This project has no MySQL or MariaDB database."))?;
        if let Some(error) = info.error {
            return Err(Failure::new(label, error));
        }
        let server = match info.source {
            database::DatabaseSource::Config => {
                let url = target
                    .database
                    .as_ref()
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                return database::config_launch(url, &target.name, &target.dir, home)
                    .map_err(|e| Failure::new(label, e));
            }
            database::DatabaseSource::Ddev => Server::Ddev,
            database::DatabaseSource::WpEnv => Server::WpEnv,
        };
        set_up(server, &target.dir)?;
        self.ensure_database_server(server, target, settings)?;
        let connection = self
            .tools
            .connection(server, &target.dir)
            .map_err(|e| Failure::new(label, e))?;
        let name = format!("{} ({})", target.name, server.label());
        Ok(database::server_launch(&connection, &name))
    }
}

/// Refused, with what's missing, when the folder isn't set up for the server.
fn set_up(server: Server, dir: &Path) -> Result<(), Failure> {
    match server.missing_setup(dir) {
        Some(missing) => Err(Failure::new(format!("Start {}", server.label()), missing)),
        None => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use serde_json::json;
    use tempfile::TempDir;

    use super::database::{DatabaseSource, Launch};
    use super::tools::StandIn;
    use super::*;

    const DDEV_CONFIG: &str = "name: demo-site\ntype: php\n";
    const NO_DDEV_CONFIG: &str = "No .ddev/config.yaml: run “ddev config” in the project.";

    struct Fx {
        root: TempDir,
        dev: DevEnv,
        changes: Arc<AtomicUsize>,
        settings: Settings,
    }

    impl Fx {
        fn new() -> Self {
            let changes = Arc::new(AtomicUsize::new(0));
            let counter = changes.clone();
            let dev = DevEnv::new(
                Box::new(StandIn::instant()),
                Box::new(move || {
                    counter.fetch_add(1, Ordering::SeqCst);
                }),
            );
            Fx {
                root: tempfile::tempdir().unwrap(),
                dev,
                changes,
                settings: Settings::default(),
            }
        }

        /// A project whose folder is `slug`, with these files in it.
        fn project(
            &self,
            slug: &str,
            mode: DevMode,
            files: &[(&str, &str)],
            database: Option<Value>,
        ) -> Target {
            let dir = self.root.path().join(slug);
            fs::create_dir_all(&dir).unwrap();
            for (path, text) in files {
                let file = dir.join(path);
                fs::create_dir_all(file.parent().unwrap()).unwrap();
                fs::write(file, text).unwrap();
            }
            Target {
                id: slug.into(),
                name: "Demo Site".into(),
                dir,
                env: Vec::new(),
                mode,
                database,
            }
        }

        fn ddev(&self) -> Target {
            self.project(
                "demo-site",
                DevMode::Ddev,
                &[(".ddev/config.yaml", DDEV_CONFIG)],
                None,
            )
        }

        fn status(&self, target: &Target) -> DevStatus {
            self.dev.status(target, &self.settings)
        }

        fn act(&self, target: &Target, action: DevAction) -> Result<(), Failure> {
            self.dev.act(target, action, &self.settings)
        }

        fn server_state(&self, target: &Target) -> ServerState {
            self.status(target).server.unwrap().state
        }

        fn docker_running(&self, target: &Target) -> bool {
            self.status(target).docker.unwrap().running
        }

        fn changes(&self) -> usize {
            self.changes.load(Ordering::SeqCst)
        }
    }

    fn failure(label: &str, output: &str) -> Failure {
        Failure {
            label: label.into(),
            output: output.into(),
        }
    }

    #[test]
    fn a_project_without_a_dev_mode_or_database_has_an_empty_status() {
        let fx = Fx::new();
        let target = fx.project("plain-site", DevMode::None, &[], None);

        assert_eq!(fx.status(&target), DevStatus::default());
    }

    #[test]
    fn a_project_with_only_a_database_url_shows_the_database_and_no_docker() {
        let fx = Fx::new();
        let target = fx.project(
            "plain-site",
            DevMode::None,
            &[],
            Some(json!("mysql://admin:pw@db.example.com/shop")),
        );

        assert_eq!(
            fx.status(&target),
            DevStatus {
                docker: None,
                server: None,
                database: Some(DatabaseInfo {
                    source: DatabaseSource::Config,
                    engine: "MySQL".into(),
                    detail: Some("admin@db.example.com/shop".into()),
                    error: None,
                }),
            }
        );
    }

    #[test]
    fn a_project_with_a_ddev_database_but_no_dev_mode_shows_docker() {
        let fx = Fx::new();
        let target = fx.project(
            "plain-site",
            DevMode::None,
            &[(".ddev/config.yaml", DDEV_CONFIG)],
            None,
        );

        let status = fx.status(&target);

        assert_eq!(status.docker.map(|d| d.running), Some(true));
        assert_eq!(status.server, None);
        assert_eq!(
            status.database.map(|d| d.source),
            Some(DatabaseSource::Ddev)
        );
    }

    #[test]
    fn a_ddev_project_is_stopped_on_a_running_docker_and_runs_after_its_server_starts() {
        let fx = Fx::new();
        let target = fx.ddev();

        assert_eq!(
            fx.status(&target),
            DevStatus {
                docker: Some(DockerStatus {
                    running: true,
                    activity: None,
                    manager: "Docker Desktop".into(),
                }),
                server: Some(ServerStatus {
                    server: Server::Ddev,
                    label: "DDEV".into(),
                    state: ServerState::Stopped,
                    url: None,
                    detail: None,
                    activity: None,
                }),
                database: Some(DatabaseInfo {
                    source: DatabaseSource::Ddev,
                    engine: "MariaDB".into(),
                    detail: None,
                    error: None,
                }),
            }
        );

        fx.act(&target, DevAction::ServerStart).unwrap();

        let server = fx.status(&target).server.unwrap();
        assert_eq!(server.state, ServerState::Running);
        assert_eq!(server.url.as_deref(), Some("https://demo-site.ddev.site"));
        assert_eq!(server.activity, None);
        assert!(fx.changes() > 0);
    }

    #[test]
    fn running_lists_the_targets_whose_server_was_started_in_target_order() {
        let fx = Fx::new();
        let first = fx.project(
            "first-site",
            DevMode::Ddev,
            &[(".ddev/config.yaml", DDEV_CONFIG)],
            None,
        );
        let idle = fx.project(
            "idle-site",
            DevMode::Ddev,
            &[(".ddev/config.yaml", DDEV_CONFIG)],
            None,
        );
        let shop = fx.project("demo-shop", DevMode::WpEnv, &[(".wp-env.json", "{}")], None);
        let targets = [first, idle, shop];
        assert_eq!(fx.dev.running(&targets), Vec::<String>::new());

        fx.act(&targets[2], DevAction::ServerStart).unwrap();
        fx.act(&targets[0], DevAction::ServerStart).unwrap();

        assert_eq!(fx.dev.running(&targets), ["first-site", "demo-shop"]);

        fx.act(&targets[0], DevAction::ServerStop).unwrap();

        assert_eq!(fx.dev.running(&targets), ["demo-shop"]);
    }

    #[test]
    fn running_ignores_targets_without_a_dev_server() {
        let fx = Fx::new();
        let targets = [
            fx.project("compose-site", DevMode::Docker, &[], None),
            fx.ddev(),
            fx.project("plain-site", DevMode::None, &[], None),
        ];
        fx.act(&targets[1], DevAction::ServerStart).unwrap();

        assert_eq!(fx.dev.running(&targets), ["demo-site"]);
        assert_eq!(fx.dev.running(&targets[..1]), Vec::<String>::new());
        assert_eq!(fx.dev.running(&[]), Vec::<String>::new());
    }

    #[test]
    fn nothing_runs_while_docker_is_stopped() {
        let fx = Fx::new();
        let target = fx.ddev();
        fx.act(&target, DevAction::ServerStart).unwrap();
        assert_eq!(fx.dev.running(std::slice::from_ref(&target)), ["demo-site"]);

        fx.act(&target, DevAction::DockerStop).unwrap();

        assert_eq!(
            fx.dev.running(std::slice::from_ref(&target)),
            Vec::<String>::new()
        );
    }

    #[test]
    fn stopping_a_server_stops_it_again() {
        let fx = Fx::new();
        let target = fx.ddev();
        fx.act(&target, DevAction::ServerStart).unwrap();

        fx.act(&target, DevAction::ServerStop).unwrap();

        assert_eq!(fx.server_state(&target), ServerState::Stopped);
    }

    #[test]
    fn restarting_a_server_leaves_it_running() {
        let fx = Fx::new();
        let target = fx.ddev();

        fx.act(&target, DevAction::ServerRestart).unwrap();

        assert_eq!(fx.server_state(&target), ServerState::Running);
    }

    #[test]
    fn a_wp_env_project_runs_on_its_stand_in_url() {
        let fx = Fx::new();
        let target = fx.project("demo-shop", DevMode::WpEnv, &[(".wp-env.json", "{}")], None);

        fx.act(&target, DevAction::ServerStart).unwrap();

        let server = fx.status(&target).server.unwrap();
        assert_eq!(server.server, Server::WpEnv);
        assert_eq!(server.label, "wp-env");
        assert_eq!(server.state, ServerState::Running);
        assert_eq!(server.url.as_deref(), Some("http://localhost:8888"));
    }

    #[test]
    fn a_ddev_project_without_its_config_is_not_configured_and_cannot_start() {
        let fx = Fx::new();
        let target = fx.project("demo-site", DevMode::Ddev, &[], None);

        let server = fx.status(&target).server.unwrap();
        assert_eq!(
            server,
            ServerStatus {
                server: Server::Ddev,
                label: "DDEV".into(),
                state: ServerState::NotConfigured,
                url: None,
                detail: Some(NO_DDEV_CONFIG.into()),
                activity: None,
            }
        );
        assert_eq!(
            fx.act(&target, DevAction::ServerStart),
            Err(failure("Start DDEV", NO_DDEV_CONFIG))
        );
    }

    #[test]
    fn a_wp_env_project_without_its_config_says_which_file_is_missing() {
        let fx = Fx::new();
        let target = fx.project("demo-shop", DevMode::WpEnv, &[], None);

        let server = fx.status(&target).server.unwrap();

        assert_eq!(server.state, ServerState::NotConfigured);
        assert_eq!(
            server.detail.as_deref(),
            Some("No .wp-env.json in this folder.")
        );
        assert_eq!(
            fx.act(&target, DevAction::ServerStart),
            Err(failure("Start wp-env", "No .wp-env.json in this folder."))
        );
    }

    #[test]
    fn a_server_action_on_a_project_without_a_dev_server_fails() {
        let fx = Fx::new();
        let target = fx.project("plain-site", DevMode::Docker, &[], None);

        for action in [
            DevAction::ServerStart,
            DevAction::ServerStop,
            DevAction::ServerRestart,
        ] {
            assert_eq!(
                fx.act(&target, action),
                Err(failure(
                    "Dev environment",
                    "This project has no dev server (DDEV, wp-env) set up."
                )),
                "{action:?}"
            );
        }
    }

    #[test]
    fn stopping_docker_stops_it_and_every_server_with_it() {
        let fx = Fx::new();
        let target = fx.ddev();
        fx.act(&target, DevAction::ServerStart).unwrap();

        fx.act(&target, DevAction::DockerStop).unwrap();

        let status = fx.status(&target);
        assert_eq!(status.docker.map(|d| d.running), Some(false));
        assert_eq!(status.server.map(|s| s.state), Some(ServerState::Stopped));
    }

    #[test]
    fn starting_a_server_starts_docker_first() {
        let fx = Fx::new();
        let target = fx.ddev();
        fx.act(&target, DevAction::DockerStop).unwrap();
        assert!(!fx.docker_running(&target));

        fx.act(&target, DevAction::ServerStart).unwrap();

        assert!(fx.docker_running(&target));
        assert_eq!(fx.server_state(&target), ServerState::Running);
    }

    #[test]
    fn docker_start_and_restart_leave_docker_running() {
        let fx = Fx::new();
        let target = fx.ddev();
        fx.act(&target, DevAction::DockerStop).unwrap();

        fx.act(&target, DevAction::DockerStart).unwrap();
        assert!(fx.docker_running(&target));

        fx.act(&target, DevAction::DockerRestart).unwrap();
        assert!(fx.docker_running(&target));
    }

    #[test]
    fn preparing_starts_the_server_once_per_session() {
        let fx = Fx::new();
        let target = fx.ddev();

        fx.dev.prepare(&target, &fx.settings).unwrap();
        assert_eq!(fx.server_state(&target), ServerState::Running);

        fx.act(&target, DevAction::ServerStop).unwrap();
        fx.dev.prepare(&target, &fx.settings).unwrap();

        assert_eq!(fx.server_state(&target), ServerState::Stopped);
    }

    #[test]
    fn a_server_stopped_by_hand_before_the_project_opens_stays_stopped() {
        let fx = Fx::new();
        let target = fx.ddev();
        fx.act(&target, DevAction::ServerStop).unwrap();

        fx.dev.prepare(&target, &fx.settings).unwrap();

        assert_eq!(fx.server_state(&target), ServerState::Stopped);
    }

    #[test]
    fn preparing_a_project_without_a_dev_mode_does_nothing() {
        let fx = Fx::new();
        let target = fx.project(
            "plain-site",
            DevMode::None,
            &[(".ddev/config.yaml", DDEV_CONFIG)],
            None,
        );
        let other = fx.ddev();
        fx.act(&other, DevAction::DockerStop).unwrap();
        let before = fx.changes();

        fx.dev.prepare(&target, &fx.settings).unwrap();

        assert!(!fx.docker_running(&other));
        assert_eq!(fx.changes(), before);
    }

    #[test]
    fn preparing_a_docker_project_starts_docker_without_a_server() {
        let fx = Fx::new();
        let other = fx.ddev();
        fx.act(&other, DevAction::DockerStop).unwrap();
        let target = fx.project("compose-site", DevMode::Docker, &[], None);

        fx.dev.prepare(&target, &fx.settings).unwrap();

        assert!(fx.docker_running(&target));
        assert_eq!(fx.status(&target).server, None);
    }

    #[test]
    fn preparing_a_ddev_project_without_its_config_only_starts_docker() {
        let fx = Fx::new();
        let stopper = fx.project("demo-site", DevMode::Ddev, &[], None);
        fx.act(&stopper, DevAction::DockerStop).unwrap();
        let fresh = fx.project("fresh-site", DevMode::Ddev, &[], None);

        fx.dev.prepare(&fresh, &fx.settings).unwrap();

        assert!(fx.docker_running(&fresh));
        assert_eq!(fx.server_state(&fresh), ServerState::NotConfigured);
    }

    #[test]
    fn a_ddev_database_launch_starts_the_stopped_server_and_connects_to_it() {
        let fx = Fx::new();
        let target = fx.ddev();
        assert!(!fx.dev.database_ready(&target));

        let launch = fx.dev.database_launch(&target, &fx.settings, None).unwrap();

        assert_eq!(
            launch,
            Launch {
                args: [
                    "-H",
                    "127.0.0.1",
                    "-P",
                    "32768",
                    "-u",
                    "db",
                    "-d",
                    "db",
                    "--name",
                    "Demo Site (DDEV)"
                ]
                .map(String::from)
                .to_vec(),
                password: "db".into(),
            }
        );
        assert_eq!(fx.server_state(&target), ServerState::Running);
        assert!(fx.dev.database_ready(&target));
    }

    #[test]
    fn a_wp_env_database_launch_names_the_server_in_the_connection() {
        let fx = Fx::new();
        let target = fx.project("demo-shop", DevMode::WpEnv, &[(".wp-env.json", "{}")], None);

        let launch = fx.dev.database_launch(&target, &fx.settings, None).unwrap();

        assert_eq!(
            launch.args.last().map(String::as_str),
            Some("Demo Site (wp-env)")
        );
        assert_eq!(fx.server_state(&target), ServerState::Running);
    }

    #[test]
    fn a_url_database_launches_from_the_url_without_starting_anything() {
        let fx = Fx::new();
        let target = fx.project(
            "demo-site",
            DevMode::Ddev,
            &[(".ddev/config.yaml", DDEV_CONFIG)],
            Some(json!("mysql://admin:pw@db.example.com/shop")),
        );

        let launch = fx
            .dev
            .database_launch(&target, &fx.settings, Some(Path::new("/home/demo")))
            .unwrap();

        assert_eq!(
            launch,
            Launch {
                args: ["mysql://admin@db.example.com/shop", "--name", "Demo Site"]
                    .map(String::from)
                    .to_vec(),
                password: "pw".into(),
            }
        );
        assert_eq!(fx.server_state(&target), ServerState::Stopped);
        assert!(fx.dev.database_ready(&target));
    }

    #[test]
    fn a_url_database_with_a_socket_resolves_the_home_folder() {
        let fx = Fx::new();
        let target = fx.project(
            "plain-site",
            DevMode::None,
            &[],
            Some(json!("mysql://app@localhost/shop?socket=~/run/mysql.sock")),
        );

        let launch = fx
            .dev
            .database_launch(&target, &fx.settings, Some(Path::new("/home/demo")))
            .unwrap();

        assert_eq!(
            launch.args,
            [
                "mysql://app@localhost/shop",
                "--socket",
                "/home/demo/run/mysql.sock",
                "--name",
                "Demo Site"
            ]
        );
    }

    #[test]
    fn a_project_without_a_database_cannot_open_one() {
        let fx = Fx::new();
        let target = fx.project("plain-site", DevMode::None, &[], None);

        assert_eq!(
            fx.dev.database_launch(&target, &fx.settings, None),
            Err(failure(
                "Open Database",
                "This project has no MySQL or MariaDB database."
            ))
        );
    }

    #[test]
    fn a_database_that_is_not_a_url_cannot_be_opened() {
        let fx = Fx::new();
        let target = fx.project("plain-site", DevMode::None, &[], Some(json!("not a url")));

        assert_eq!(
            fx.dev.database_launch(&target, &fx.settings, None),
            Err(failure(
                "Open Database",
                "“database” must be a mysql:// URL."
            ))
        );
    }

    #[test]
    fn a_hidden_database_cannot_be_opened() {
        let fx = Fx::new();
        let target = fx.project(
            "demo-site",
            DevMode::Ddev,
            &[(".ddev/config.yaml", DDEV_CONFIG)],
            Some(json!(false)),
        );

        assert_eq!(fx.status(&target).database, None);
        assert_eq!(
            fx.dev.database_launch(&target, &fx.settings, None),
            Err(failure(
                "Open Database",
                "This project has no MySQL or MariaDB database."
            ))
        );
    }

    #[test]
    fn the_docker_managers_name_shows_in_the_status() {
        let mut fx = Fx::new();
        fx.settings.docker_manager = crate::store::DockerManager::OrbStack;
        let target = fx.ddev();

        assert_eq!(
            fx.status(&target).docker.map(|d| d.manager),
            Some("OrbStack".to_string())
        );
    }
}
