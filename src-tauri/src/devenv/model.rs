//! The dev environment's vocabulary: servers, states, actions, the status the detail shows, and the project a
//! call is about.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use ts_rs::TS;

use super::database::{self, DatabaseInfo};
use super::wpenv;
use crate::projects::config::DevMode;

/// A tool that runs a project's local server inside Docker.
#[derive(Serialize, Deserialize, TS, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "kebab-case")]
pub enum Server {
    Ddev,
    WpEnv,
}

impl Server {
    pub fn of(mode: DevMode) -> Option<Self> {
        match mode {
            DevMode::Ddev => Some(Server::Ddev),
            DevMode::WpEnv => Some(Server::WpEnv),
            DevMode::Docker | DevMode::None => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Server::Ddev => "DDEV",
            Server::WpEnv => "wp-env",
        }
    }

    /// Why the folder can't use the tool, or `None` when it is set up.
    pub fn missing_setup(self, dir: &Path) -> Option<String> {
        match self {
            Server::Ddev if !dir.join(".ddev/config.yaml").is_file() => {
                Some("No .ddev/config.yaml: run “ddev config” in the project.".into())
            }
            Server::WpEnv if !wpenv::configured(dir) => {
                Some(format!("No {} in this folder.", wpenv::CONFIG_FILE))
            }
            _ => None,
        }
    }
}

#[derive(Serialize, TS, Debug, Clone, Copy, PartialEq, Eq)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub enum ServerState {
    Running,
    Paused,
    Stopped,
    NotConfigured,
    Unknown,
}

/// What is under way right now.
#[derive(Serialize, TS, Debug, Clone, Copy, PartialEq, Eq)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub enum Activity {
    Starting,
    Stopping,
    Restarting,
}

#[derive(Serialize, Deserialize, TS, Debug, Clone, Copy, PartialEq, Eq)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub enum DevAction {
    DockerStart,
    DockerStop,
    DockerRestart,
    ServerStart,
    ServerStop,
    ServerRestart,
}

/// A server's state as its tool describes it.
#[derive(Debug, Clone, PartialEq)]
pub struct Described {
    pub state: ServerState,
    pub url: Option<String>,
    pub detail: Option<String>,
}

/// How to reach a database from this Mac.
#[derive(Debug, Clone, PartialEq)]
pub struct Connection {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub password: String,
    pub database: String,
}

#[derive(Serialize, TS, Debug, Clone, PartialEq)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct DockerStatus {
    pub running: bool,
    pub activity: Option<Activity>,
    /// The manager's name ("OrbStack").
    pub manager: String,
}

#[derive(Serialize, TS, Debug, Clone, PartialEq)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct ServerStatus {
    pub server: Server,
    pub label: String,
    pub state: ServerState,
    pub url: Option<String>,
    pub detail: Option<String>,
    pub activity: Option<Activity>,
}

/// A project's dev environment as the detail shows it; `docker` only for a project that uses Docker.
#[derive(Serialize, TS, Debug, Clone, PartialEq, Default)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct DevStatus {
    pub docker: Option<DockerStatus>,
    pub server: Option<ServerStatus>,
    pub database: Option<DatabaseInfo>,
}

/// A step that failed: what it was ("Start DDEV") and what it printed.
#[derive(Serialize, TS, Debug, Clone, PartialEq)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct Failure {
    pub label: String,
    pub output: String,
}

impl Failure {
    pub(super) fn new(label: impl Into<String>, output: impl Into<String>) -> Self {
        Failure {
            label: label.into(),
            output: output.into(),
        }
    }
}

/// The project a dev-environment call is about.
pub struct Target {
    pub id: String,
    pub name: String,
    pub dir: PathBuf,
    pub env: Vec<(String, String)>,
    pub mode: DevMode,
    /// The file's `database`.
    pub database: Option<Value>,
}

impl Target {
    pub fn server(&self) -> Option<Server> {
        Server::of(self.mode)
    }

    pub fn database_info(&self) -> Option<DatabaseInfo> {
        database::detect(&self.dir, self.database.as_ref(), self.server())
    }

    pub(super) fn uses_docker(&self) -> bool {
        self.mode != DevMode::None
            || self
                .database_info()
                .is_some_and(|db| db.source != database::DatabaseSource::Config)
    }
}
