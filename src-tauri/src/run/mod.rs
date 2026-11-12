//! Launching a project's actions, its folder, path and file, and reading its states; every process goes
//! through the `System` port.

pub mod bus;
pub mod commands;
pub mod shell;
pub mod states;
pub mod system;
pub mod terminal;

#[cfg(test)]
mod tests;

use std::path::PathBuf;
use std::time::{Duration, Instant};

use serde::Serialize;
use ts_rs::TS;

use crate::data::Data;
use crate::error::{AppError, AppResult};
use crate::links;
use crate::projects::FILE_NAME;
use crate::projects::config::resolve::Action;
use crate::projects::config::{ActionSource, ProjectConfig, RunsAs};
use states::StateDisplay;
use system::{Job, System};
use terminal::{Launched, Sessions, Terminal};

const SCRIPT_TIMEOUT: Duration = Duration::from_secs(120);

/// How a script ended; `output` holds stdout and stderr as they came.
#[derive(Serialize, TS, Debug, Clone, PartialEq)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct ScriptResult {
    pub ok: bool,
    pub exit_code: Option<i32>,
    pub timed_out: bool,
    pub output: String,
    #[ts(type = "number")]
    pub duration_ms: u64,
}

impl ScriptResult {
    /// The error the command line reports for a failed script, with its output; `None` when it succeeded.
    pub fn failure(&self) -> Option<AppError> {
        if self.ok {
            return None;
        }
        let reason = if self.timed_out {
            format!("timed out after {} seconds", SCRIPT_TIMEOUT.as_secs())
        } else {
            format!("exit code {}", self.exit_code.unwrap_or(1))
        };
        let output = if self.output.is_empty() {
            String::new()
        } else {
            format!("{}\n", self.output)
        };
        Some(AppError::Launch(format!(
            "{output}The script failed ({reason})."
        )))
    }
}

/// What launching an action did: an app, link or terminal opened, or a script ran to its end.
#[derive(Serialize, TS, Debug, Clone, PartialEq)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Outcome {
    Opened { how: Launched },
    Ran { result: ScriptResult },
}

/// The project's states and what they mean for its actions: `hidden` actions name a state that holds now,
/// `running` terminal commands still run in their tab.
#[derive(Serialize, TS, Debug, Clone, PartialEq, Default)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct ProjectStatus {
    pub states: Vec<StateDisplay>,
    pub hidden: Vec<ActionSource>,
    pub running: Vec<ActionSource>,
}

/// Launches what a project's file names. Everything that opens something records the project as opened.
pub struct Runner<'a> {
    pub data: &'a Data,
    pub system: &'a dyn System,
    /// Where terminal scripts and sessions live.
    pub folder: PathBuf,
}

/// The project a launch works on.
struct Target {
    dir: PathBuf,
    name: String,
    config: ProjectConfig,
    env: Vec<(String, String)>,
}

impl Runner<'_> {
    fn target(&self, id: &str) -> AppResult<Target> {
        let config = self.data.project_config(id)?;
        let env = config
            .env
            .iter()
            .map(|var| (var.name.clone(), var.value.clone()))
            .collect();
        Ok(Target {
            dir: PathBuf::from(self.data.project_dir(id)?),
            name: self.data.project(id)?.name,
            config,
            env,
        })
    }

    /// A terminal that opens with `app` (the settings' terminal app).
    fn terminal<'t>(&'t self, app: &'t str) -> Terminal<'t> {
        Terminal {
            system: self.system,
            folder: self.folder.clone(),
            app,
        }
    }

    fn opened(&self, id: &str, now_ms: i64) -> AppResult<()> {
        self.data.touch_last_opened(id, now_ms).map(|_| ())
    }

    fn open_shell(&self, target: &Target) -> AppResult<()> {
        let app = self.data.settings().terminal_app;
        self.terminal(&app)
            .open(&target.name, &target.dir, &target.env, None, None)
    }

    /// Runs the action at `source`: an app on the folder (or its arguments), a link, a terminal command, or a
    /// script in a login shell whose output comes back.
    pub fn launch(&self, id: &str, source: ActionSource, now_ms: i64) -> AppResult<Outcome> {
        let target = self.target(id)?;
        let action = actions(&target.config)
            .find(|a| a.edit.source == source)
            .ok_or_else(|| {
                AppError::NotFound("That action isn’t in the project file anymore".into())
            })?;
        self.opened(id, now_ms)?;
        let how = match action.runs_as {
            RunsAs::App => {
                let app = action.app.as_deref().unwrap_or_default();
                let folder = target.dir.to_string_lossy();
                let on = action.args.as_deref().unwrap_or(&folder);
                self.system.spawn(&Job {
                    command: shell::open_with(app, on),
                    cwd: Some(target.dir.clone()),
                    env: target.env.clone(),
                })?;
                Launched::Launched
            }
            RunsAs::Url => {
                let url = action.url.as_deref().unwrap_or_default();
                if !links::is_web_url(url) {
                    return Err(AppError::InvalidInput(format!("Not a web link: {url}")));
                }
                self.system.open(&[url])?;
                Launched::Launched
            }
            RunsAs::Terminal => match command_of(action) {
                Some(command) => {
                    let app = self.data.settings().terminal_app;
                    self.terminal(&app)
                        .run(id, &target.name, &target.dir, &target.env, command)?
                }
                None => {
                    self.open_shell(&target)?;
                    Launched::Launched
                }
            },
            RunsAs::Script => {
                return Ok(Outcome::Ran {
                    result: self.script(&target, action.command.as_deref().unwrap_or_default()),
                });
            }
        };
        Ok(Outcome::Opened { how })
    }

    fn script(&self, target: &Target, command: &str) -> ScriptResult {
        let start = Instant::now();
        let output = self.system.run(
            &Job {
                command: command.into(),
                cwd: Some(target.dir.clone()),
                env: target.env.clone(),
            },
            SCRIPT_TIMEOUT,
        );
        ScriptResult {
            ok: output.ok(),
            exit_code: output.code,
            timed_out: output.timed_out,
            output: output.output.trim_end().to_string(),
            duration_ms: start.elapsed().as_millis() as u64,
        }
    }

    /// A shell in the project folder with its env.
    pub fn open_terminal(&self, id: &str, now_ms: i64) -> AppResult<()> {
        let target = self.target(id)?;
        self.opened(id, now_ms)?;
        self.open_shell(&target)
    }

    /// Runs `command` in a terminal in the project folder; one still running there comes to the front.
    pub fn terminal_command(&self, id: &str, command: &str, now_ms: i64) -> AppResult<Launched> {
        let target = self.target(id)?;
        self.opened(id, now_ms)?;
        let app = self.data.settings().terminal_app;
        self.terminal(&app)
            .run(id, &target.name, &target.dir, &target.env, command)
    }

    /// Opens a file of the project (`path` relative to its folder, never outside it) in the project's editor,
    /// else the default editor, else its default app.
    pub fn open_file(&self, id: &str, path: &str, now_ms: i64) -> AppResult<()> {
        let relative = std::path::Path::new(path);
        let inside = relative.components().all(|c| {
            matches!(
                c,
                std::path::Component::Normal(_) | std::path::Component::CurDir
            )
        });
        if !inside {
            return Err(AppError::InvalidInput(format!(
                "Not a file of the project: {path}"
            )));
        }
        let dir = PathBuf::from(self.data.project_dir(id)?);
        let file = dir.join(relative);
        if !file.is_file() {
            return Err(AppError::NotFound(format!("{path} isn’t there anymore")));
        }
        let editor = self
            .data
            .project(id)?
            .meta
            .editor
            .unwrap_or(self.data.settings().default_editor);
        self.opened(id, now_ms)?;
        let file = file.to_string_lossy();
        match editor.trim() {
            "" => self.system.open(&[&file]),
            editor => self.system.spawn(&Job {
                command: shell::open_with(editor, &file),
                cwd: Some(dir.clone()),
                env: Vec::new(),
            }),
        }
    }

    pub fn reveal(&self, id: &str, now_ms: i64) -> AppResult<()> {
        let dir = self.data.project_dir(id)?;
        self.opened(id, now_ms)?;
        self.system.open(&[&dir])
    }

    pub fn copy_path(&self, id: &str) -> AppResult<()> {
        self.system.copy(&self.data.project_dir(id)?)
    }

    /// Opens `.project-launcher.json` in the settings' config editor, else the project's or the default
    /// editor, else the default text editor.
    pub fn edit_file(&self, id: &str, now_ms: i64) -> AppResult<()> {
        let project = self.data.project(id)?;
        let dir = PathBuf::from(self.data.project_dir(id)?);
        let file = dir.join(FILE_NAME);
        if !file.is_file() {
            return Err(AppError::NotFound(format!(
                "{} has no {FILE_NAME}",
                project.name
            )));
        }
        let settings = self.data.settings();
        let editor = [
            settings.config_editor,
            project.meta.editor.unwrap_or_default(),
            settings.default_editor,
        ]
        .into_iter()
        .map(|e| e.trim().to_string())
        .find(|e| !e.is_empty());
        self.opened(id, now_ms)?;
        let path = file.to_string_lossy();
        match editor {
            Some(editor) => self.system.spawn(&Job {
                command: shell::open_with(&editor, &path),
                cwd: Some(dir.clone()),
                env: Vec::new(),
            }),
            None => self.system.open(&["-t", &path]),
        }
    }

    pub fn status(&self, id: &str) -> AppResult<ProjectStatus> {
        let target = self.target(id)?;
        let config = &target.config;
        let states = states::resolve(
            self.system,
            &target.dir,
            &target.env,
            &config.state_providers,
            &config.states,
        );
        Ok(ProjectStatus {
            hidden: hidden_by(config, &states.keys),
            running: self.running_terminals(id, config),
            states: states.display,
        })
    }

    /// Terminal commands whose session still runs in its tab.
    fn running_terminals(&self, id: &str, config: &ProjectConfig) -> Vec<ActionSource> {
        let sessions = Sessions::in_folder(&self.folder);
        config
            .apps
            .iter()
            .filter(|a| a.runs_as == RunsAs::Terminal)
            .filter(|a| command_of(a).is_some_and(|c| sessions.is_running(self.system, id, c)))
            .map(|a| a.edit.source)
            .collect()
    }
}

fn actions(config: &ProjectConfig) -> impl Iterator<Item = &Action> {
    config.apps.iter().chain(&config.scripts)
}

/// The actions whose `hiddenStates` name one of the state `keys` that hold now.
fn hidden_by(config: &ProjectConfig, keys: &[String]) -> Vec<ActionSource> {
    actions(config)
        .filter(|a| a.hidden_states.iter().any(|s| keys.contains(s)))
        .map(|a| a.edit.source)
        .collect()
}

/// The action's command when it has text.
fn command_of(action: &Action) -> Option<&str> {
    action.command.as_deref().filter(|c| !c.trim().is_empty())
}
