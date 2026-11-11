//! The `launcher` command line. Without a subcommand it starts the app (or activates the running one);
//! a subcommand asks the running app over the socket and prints its answer (`reply`). A new subcommand adds a
//! `Command` variant, its arm in `message`, its `IpcMessage`, its arm in `remote::handle`, a place in `parse`'s
//! `starts_app` when it may start the app, and an arm in `reply::render` when it prints something.

pub mod reply;

use std::ffi::OsString;
use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::ipc::IpcMessage;

/// The `--log-level` of the CLI; without it the app logs at `Info`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, clap::ValueEnum)]
pub enum LogLevel {
    Error,
    Warn,
    #[default]
    Info,
    Debug,
    Trace,
}

impl From<LogLevel> for log::LevelFilter {
    fn from(level: LogLevel) -> Self {
        match level {
            LogLevel::Error => log::LevelFilter::Error,
            LogLevel::Warn => log::LevelFilter::Warn,
            LogLevel::Info => log::LevelFilter::Info,
            LogLevel::Debug => log::LevelFilter::Debug,
            LogLevel::Trace => log::LevelFilter::Trace,
        }
    }
}

const EXAMPLES: &str = "<project> is a project's name (a unique part is enough), a folder inside it (. for here) or its id.
App and script names match case-insensitively; a unique part of the name is enough.

Examples:
  launcher                           start Launcher or bring it to the front
  launcher --hidden                  start Launcher in the menu bar only
  launcher list --json               every project as JSON
  launcher launch acme editor        open Acme's editor
  launcher run . build               run this folder's project's Build script and print its output
  launcher command app:palette       run a command (see launcher commands)";

#[derive(Parser, Debug)]
#[command(
    name = "launcher",
    version,
    about = "List, search and launch your coding projects",
    disable_help_subcommand = true,
    after_help = EXAMPLES
)]
struct Cli {
    /// Start without showing the window; a running instance stays as it is
    #[arg(long)]
    hidden: bool,
    /// Log level for this run
    #[arg(long, value_enum)]
    log_level: Option<LogLevel>,
    /// Print replies as JSON
    #[arg(long, global = true)]
    json: bool,
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Whether the app runs, its version and whether its window shows
    Status,
    /// Show the window without bringing the app to the front; starts the app when it isn't running
    Show,
    /// Show the window and bring the app to the front; starts the app when it isn't running
    Focus,
    /// Hide the window; the app keeps running in the menu bar
    Hide,
    /// Quit the running app
    Quit,
    /// List every project
    List,
    /// Show a project's details, apps and scripts
    Describe {
        /// Defaults to the project of the current folder
        #[arg(default_value = ".")]
        project: String,
    },
    /// List the command ids for `launcher command`
    Commands,
    /// Show a project in Launcher
    Open { project: String },
    /// Open one of a project's apps, e.g. Editor
    Launch { project: String, app: String },
    /// Run one of a project's scripts and print its output
    Run { project: String, script: String },
    /// Open a terminal in the project
    Terminal { project: String },
    /// Show the project folder in Finder
    Finder { project: String },
    /// Edit the project's .project-launcher.json
    Config { project: String },
    /// Open the project's MySQL or MariaDB database in DataBee
    Db { project: String },
    /// Open the command palette
    Palette,
    /// Open Settings
    Settings,
    /// Run a command by id, e.g. app:palette
    #[command(name = "command")]
    Exec { id: String },
    /// Import Project Launcher's (the prototype's) data folder; shows what changes unless --yes
    Import {
        /// The folder with its projects.json and settings.json
        folder: String,
        /// Import, after checking what the run without it showed
        #[arg(long)]
        yes: bool,
    },
    /// Developer tools (debug builds only)
    #[cfg(debug_assertions)]
    #[command(subcommand)]
    Dev(DevCommand),
}

#[cfg(debug_assertions)]
#[derive(Subcommand, Debug)]
enum DevCommand {
    /// Replace the debug build's projects and settings with the demo seed
    Seed {
        /// Confirm replacing the data
        #[arg(long)]
        yes: bool,
    },
}

/// A message for the running app and how to print its reply.
#[derive(Debug, PartialEq)]
pub struct CliRequest {
    pub message: IpcMessage,
    pub json: bool,
}

#[derive(Debug, PartialEq)]
pub enum CliOutcome {
    /// Start the app, or hand the launch to the running one.
    Run {
        hidden: bool,
        log_level: Option<LogLevel>,
    },
    /// `--help`, `--version` or a usage error: print `message` and exit with `code` (0 or 2).
    Exit { message: String, code: i32 },
    /// A subcommand the running app answers; it never starts the app.
    Ask(CliRequest),
    /// A subcommand the running app answers, or the app starts with it (`show`, `focus`).
    Remote(CliRequest),
}

/// Parses the full argv (including the program name).
pub fn parse<I, S>(args: I) -> CliOutcome
where
    I: IntoIterator<Item = S>,
    S: Into<OsString> + Clone,
{
    let cli = match Cli::try_parse_from(args) {
        Ok(cli) => cli,
        Err(e) => {
            return CliOutcome::Exit {
                message: e.render().to_string(),
                code: e.exit_code(),
            };
        }
    };
    let Some(command) = cli.command else {
        return CliOutcome::Run {
            hidden: cli.hidden,
            log_level: cli.log_level,
        };
    };
    let starts_app = matches!(command, Command::Show | Command::Focus);
    let request = CliRequest {
        message: message(command),
        json: cli.json,
    };
    if starts_app {
        CliOutcome::Remote(request)
    } else {
        CliOutcome::Ask(request)
    }
}

fn expand_home(spec: &str) -> PathBuf {
    match (spec.strip_prefix('~'), std::env::home_dir()) {
        (Some(rest), Some(home)) => home.join(rest.trim_start_matches('/')),
        _ => PathBuf::from(spec),
    }
}

/// The folder as an absolute, canonical path, since the app runs in another working folder.
fn absolute_path(spec: &str) -> String {
    let expanded = expand_home(spec);
    let absolute = if expanded.is_absolute() {
        expanded
    } else {
        std::env::current_dir()
            .map(|cwd| cwd.join(&expanded))
            .unwrap_or(expanded)
    };
    absolute
        .canonicalize()
        .unwrap_or(absolute)
        .to_string_lossy()
        .into_owned()
}

/// A project given as a folder (`.`, `..`, `~/…`, or a path with `/`) becomes an absolute path; a name or an
/// id stays as it is.
fn project_spec(spec: String) -> String {
    let looks_like_path =
        spec == "." || spec == ".." || spec.contains('/') || spec.starts_with('~');
    if looks_like_path {
        absolute_path(&spec)
    } else {
        spec
    }
}

fn message(command: Command) -> IpcMessage {
    match command {
        Command::List => IpcMessage::List,
        Command::Describe { project } => IpcMessage::Describe {
            project: project_spec(project),
        },
        Command::Commands => IpcMessage::Commands,
        Command::Open { project } => IpcMessage::Open {
            project: project_spec(project),
        },
        Command::Launch { project, app } => IpcMessage::Launch {
            project: project_spec(project),
            name: app,
        },
        Command::Run { project, script } => IpcMessage::Run {
            project: project_spec(project),
            name: script,
        },
        Command::Terminal { project } => IpcMessage::Terminal {
            project: project_spec(project),
        },
        Command::Finder { project } => IpcMessage::Finder {
            project: project_spec(project),
        },
        Command::Config { project } => IpcMessage::Config {
            project: project_spec(project),
        },
        Command::Db { project } => IpcMessage::Db {
            project: project_spec(project),
        },
        Command::Palette => IpcMessage::Palette,
        Command::Settings => IpcMessage::Settings,
        Command::Exec { id } => IpcMessage::Command { id },
        Command::Import { folder, yes } => IpcMessage::Import {
            folder: absolute_path(&folder),
            yes,
        },
        Command::Status => IpcMessage::Status,
        Command::Show => IpcMessage::Show { focus: false },
        Command::Focus => IpcMessage::Activate { hidden: false },
        Command::Hide => IpcMessage::Hide,
        Command::Quit => IpcMessage::Quit,
        #[cfg(debug_assertions)]
        Command::Dev(DevCommand::Seed { yes }) => IpcMessage::DevSeed { yes },
    }
}

#[cfg(test)]
mod tests;
