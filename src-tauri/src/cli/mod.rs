//! The `launcher` command line. Without a subcommand it starts the app (or activates the running one);
//! a subcommand asks the running app over the socket and prints its answer (`reply`). A new subcommand adds a
//! `Command` variant, its arm in `message`, its `IpcMessage`, its arm in `remote::handle`, a place in `parse`'s
//! `starts_app` when it may start the app, and an arm in `reply::render` when it prints something.

pub mod reply;

use std::ffi::OsString;

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

const EXAMPLES: &str = "Examples:
  launcher                           start Launcher or bring it to the front
  launcher --hidden                  start Launcher in the menu bar only
  launcher show                      show the window without bringing Launcher to the front
  launcher status --json";

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

fn message(command: Command) -> IpcMessage {
    match command {
        Command::Status => IpcMessage::Status,
        Command::Show => IpcMessage::Show { focus: false },
        Command::Focus => IpcMessage::Activate { hidden: false },
        Command::Hide => IpcMessage::Hide,
        Command::Quit => IpcMessage::Quit,
    }
}

#[cfg(test)]
mod tests;
