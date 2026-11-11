//! `Runner` through `Data`: what launching an action, the folder, the path and the project file do, and the
//! status the detail shows.

use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;

use serde_json::{Value, json};
use tempfile::TempDir;

use super::*;
use crate::error::{AppError, AppResult, ErrorKind};
use crate::projects::config::ActionList;
use crate::projects::{KnownProject, Registry};
use crate::run::shell::open_with;
use crate::run::system::{Output, Recorder};
use crate::store::Settings;
use crate::test_support::temp_config_dir;

const NOW: i64 = 5_000;

fn apps(index: usize) -> ActionSource {
    ActionSource {
        list: ActionList::Apps,
        index,
    }
}

fn scripts(index: usize) -> ActionSource {
    ActionSource {
        list: ActionList::Scripts,
        index,
    }
}

/// The apps: 0 Sketchpad on the folder, 1 Sketchpad on a file, 2 a link, 3 a dev server in a terminal, 4 a
/// plain terminal. Scripts: 0 build, 1 deploy.
fn project_file() -> Value {
    json!({
        "name": "Northwind",
        "env": { "STAGE": "demo", "PORT": 8080 },
        "apps": [
            { "label": "Sketch", "app": "Sketchpad" },
            { "label": "Notes", "app": "Sketchpad", "args": "${dir}/notes.txt" },
            { "label": "Docs", "url": "https://example.com/docs" },
            { "label": "Dev server", "command": "npm run dev", "hiddenStates": ["live"] },
            { "label": "Shell", "command": "" },
        ],
        "scripts": [
            { "label": "Build", "command": "make build", "hiddenStates": ["live"] },
            { "label": "Deploy", "command": "make deploy", "hiddenStates": ["down"] },
        ],
        "stateProviders": { "web": "curl-demo ${dir}" },
        "states": {
            "site": {
                "source": "web",
                "label": "Site",
                "values": {
                    "live": { "value": "up", "label": "Online", "color": "green" },
                    "down": { "value": "down", "label": "Offline" },
                },
            },
        },
    })
}

struct Fx {
    _config: TempDir,
    root: TempDir,
    terminal: TempDir,
    data: Data,
    system: Recorder,
}

impl Fx {
    fn new() -> Self {
        Self::with(Some(project_file()), Settings::default())
    }

    fn with(file: Option<Value>, settings: Settings) -> Self {
        let config = temp_config_dir();
        let root = tempfile::tempdir().unwrap();
        let data = Data::open_with_home(config.path(), Some(root.path().to_path_buf()));
        let dir = root.path().join("p");
        fs::create_dir(&dir).unwrap();
        if let Some(file) = file {
            fs::write(
                dir.join(FILE_NAME),
                serde_json::to_string_pretty(&file).unwrap(),
            )
            .unwrap();
        }
        data.reset(
            settings,
            Registry {
                projects: vec![KnownProject {
                    id: "p".into(),
                    path: dir.to_string_lossy().into_owned(),
                    added_at: 1000,
                    last_opened_at: None,
                }],
            },
        )
        .unwrap();
        Self {
            _config: config,
            root,
            terminal: tempfile::tempdir().unwrap(),
            data,
            system: Recorder::default(),
        }
    }

    fn runner(&self) -> Runner<'_> {
        Runner {
            data: &self.data,
            system: &self.system,
            folder: self.terminal.path().to_path_buf(),
        }
    }

    fn dir(&self) -> PathBuf {
        self.root.path().join("p")
    }

    fn dir_text(&self) -> String {
        self.dir().to_string_lossy().into_owned()
    }

    fn file_text(&self) -> String {
        self.dir().join(FILE_NAME).to_string_lossy().into_owned()
    }

    fn last_opened_at(&self) -> Option<i64> {
        self.data.project("p").unwrap().last_opened_at
    }

    fn scripts(&self) -> Vec<PathBuf> {
        let mut files: Vec<PathBuf> = fs::read_dir(self.terminal.path())
            .unwrap()
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|e| e == "command"))
            .collect();
        files.sort();
        files
    }

    /// Records the session the last terminal script would write: pid 4242 on `/dev/ttys004`.
    fn record_session(&self) {
        let script = fs::read_to_string(self.scripts().last().unwrap()).unwrap();
        let line = script.lines().find(|l| l.starts_with("printf")).unwrap();
        let path = line.rsplit_once("> ").unwrap().1.trim_matches('\'');
        fs::write(path, r#"{"pid":4242,"tty":"/dev/ttys004"}"#).unwrap();
    }
}

fn settings_with(terminal: &str, config_editor: &str, default_editor: &str) -> Settings {
    Settings {
        terminal_app: terminal.into(),
        config_editor: config_editor.into(),
        default_editor: default_editor.into(),
        ..Settings::default()
    }
}

/// Keeps every job it is given, to check the folder and env they run with.
#[derive(Default)]
struct JobSpy {
    jobs: Mutex<Vec<Job>>,
}

impl System for JobSpy {
    fn spawn(&self, job: &Job) -> AppResult<()> {
        self.jobs.lock().unwrap().push(job.clone());
        Ok(())
    }

    fn run(&self, job: &Job, _timeout: Duration) -> Output {
        self.jobs.lock().unwrap().push(job.clone());
        Output {
            code: Some(0),
            ..Output::default()
        }
    }

    fn open(&self, _args: &[&str]) -> AppResult<()> {
        Ok(())
    }

    fn tool(&self, _program: &str, _args: &[&str]) -> Option<String> {
        None
    }

    fn copy(&self, _text: &str) -> AppResult<()> {
        Ok(())
    }
}

fn expected_env() -> Vec<(String, String)> {
    vec![
        ("STAGE".into(), "demo".into()),
        ("PORT".into(), "8080".into()),
    ]
}

#[test]
fn an_app_opens_the_project_folder() {
    let fx = Fx::new();

    let outcome = fx.runner().launch("p", apps(0), NOW).unwrap();

    assert_eq!(
        outcome,
        Outcome::Opened {
            how: Launched::Launched
        }
    );
    assert_eq!(
        fx.system.calls(),
        [format!("spawn: {}", open_with("Sketchpad", &fx.dir_text()))]
    );
}

#[test]
fn an_app_with_arguments_opens_them_instead_of_the_folder() {
    let fx = Fx::new();

    fx.runner().launch("p", apps(1), NOW).unwrap();

    assert_eq!(
        fx.system.calls(),
        [format!(
            "spawn: {}",
            open_with("Sketchpad", &format!("{}/notes.txt", fx.dir_text()))
        )]
    );
}

#[test]
fn apps_and_scripts_run_in_the_project_folder_with_its_env() {
    let fx = Fx::new();
    let spy = JobSpy::default();
    let runner = Runner {
        data: &fx.data,
        system: &spy,
        folder: fx.terminal.path().to_path_buf(),
    };

    runner.launch("p", apps(0), NOW).unwrap();
    runner.launch("p", scripts(0), NOW).unwrap();

    assert_eq!(
        *spy.jobs.lock().unwrap(),
        [
            Job {
                command: open_with("Sketchpad", &fx.dir_text()),
                cwd: Some(fx.dir()),
                env: expected_env(),
            },
            Job {
                command: "make build".into(),
                cwd: Some(fx.dir()),
                env: expected_env(),
            },
        ]
    );
}

#[test]
fn a_link_is_opened() {
    let fx = Fx::new();

    let outcome = fx.runner().launch("p", apps(2), NOW).unwrap();

    assert_eq!(
        outcome,
        Outcome::Opened {
            how: Launched::Launched
        }
    );
    assert_eq!(fx.system.calls(), ["open: https://example.com/docs"]);
}

#[test]
fn a_script_returns_its_trimmed_output_and_exit_status() {
    let fx = Fx::new();
    fx.system.reply("make build", "built in 2s\n\n");

    let outcome = fx.runner().launch("p", scripts(0), NOW).unwrap();

    let Outcome::Ran { result } = outcome else {
        panic!("a script runs to its end: {outcome:?}");
    };
    assert!(result.ok);
    assert_eq!(result.exit_code, Some(0));
    assert!(!result.timed_out);
    assert_eq!(result.output, "built in 2s");
}

#[test]
fn a_terminal_command_opens_a_script_in_the_terminal_app() {
    let fx = Fx::with(Some(project_file()), settings_with("Ghostty", "", ""));

    let outcome = fx.runner().launch("p", apps(3), NOW).unwrap();

    assert_eq!(
        outcome,
        Outcome::Opened {
            how: Launched::Launched
        }
    );
    let files = fx.scripts();
    assert_eq!(files.len(), 1);
    assert_eq!(
        fx.system.calls(),
        [format!("open: -a Ghostty {}", files[0].display())]
    );
    let script = fs::read_to_string(&files[0]).unwrap();
    assert!(script.contains("cd "));
    assert!(script.contains("export STAGE='demo'"));
    assert!(script.contains("export PORT='8080'"));
    assert!(script.lines().any(|l| l == "npm run dev"));
}

#[test]
fn a_terminal_action_without_a_command_opens_a_plain_shell() {
    let fx = Fx::new();

    let outcome = fx.runner().launch("p", apps(4), NOW).unwrap();

    assert_eq!(
        outcome,
        Outcome::Opened {
            how: Launched::Launched
        }
    );
    let script = fs::read_to_string(&fx.scripts()[0]).unwrap();
    assert!(!script.contains("printf"));
    assert!(fx.system.calls()[0].starts_with("open: -a Terminal "));
}

#[test]
fn launching_a_terminal_command_that_still_runs_brings_its_tab_forward() {
    let fx = Fx::with(Some(project_file()), settings_with("Ghostty", "", ""));
    fx.runner().launch("p", apps(3), NOW).unwrap();
    fx.record_session();
    fx.system.reply("ps -o tty= -p 4242", "ttys004\n");

    let outcome = fx.runner().launch("p", apps(3), NOW).unwrap();

    assert_eq!(
        outcome,
        Outcome::Opened {
            how: Launched::Activated
        }
    );
    assert_eq!(fx.scripts().len(), 1);
}

#[test]
fn launching_an_action_records_the_project_as_opened() {
    let fx = Fx::new();
    assert_eq!(fx.last_opened_at(), None);

    fx.runner().launch("p", scripts(0), NOW).unwrap();

    assert_eq!(fx.last_opened_at(), Some(NOW));
}

#[test]
fn everything_that_opens_the_project_records_it_as_opened() {
    let cases: [(&str, fn(&Runner<'_>) -> AppResult<()>); 3] = [
        ("open_terminal", |r| r.open_terminal("p", NOW)),
        ("reveal", |r| r.reveal("p", NOW)),
        ("edit_file", |r| r.edit_file("p", NOW)),
    ];
    for (name, action) in cases {
        let fx = Fx::new();

        action(&fx.runner()).unwrap();

        assert_eq!(fx.last_opened_at(), Some(NOW), "{name}");
    }
}

#[test]
fn an_action_that_is_not_in_the_file_is_not_found_and_records_nothing() {
    let fx = Fx::new();

    for source in [apps(9), scripts(2)] {
        let err = fx.runner().launch("p", source, NOW).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::NotFound, "{source:?}");
    }

    assert_eq!(fx.last_opened_at(), None);
    assert!(fx.system.calls().is_empty());
}

#[test]
fn an_unknown_project_is_not_found_everywhere() {
    let fx = Fx::new();
    let runner = fx.runner();
    let errors = [
        runner.launch("nope", apps(0), NOW).unwrap_err(),
        runner.open_terminal("nope", NOW).unwrap_err(),
        runner.reveal("nope", NOW).unwrap_err(),
        runner.copy_path("nope").unwrap_err(),
        runner.edit_file("nope", NOW).unwrap_err(),
        runner.status("nope").unwrap_err(),
    ];
    for err in errors {
        assert!(matches!(err, AppError::NotFound(_)), "{err:?}");
    }
}

#[test]
fn open_terminal_opens_a_shell_in_the_project_folder() {
    let fx = Fx::with(Some(project_file()), settings_with("Ghostty", "", ""));

    fx.runner().open_terminal("p", NOW).unwrap();

    let files = fx.scripts();
    assert_eq!(
        fx.system.calls(),
        [format!("open: -a Ghostty {}", files[0].display())]
    );
    let script = fs::read_to_string(&files[0]).unwrap();
    assert!(script.contains(&format!("cd '{}'", fx.dir_text())));
    assert!(script.contains("export STAGE='demo'"));
}

#[test]
fn copy_path_puts_the_project_folder_on_the_clipboard() {
    let fx = Fx::new();

    fx.runner().copy_path("p").unwrap();

    assert_eq!(fx.system.clipboard(), Some(fx.dir_text()));
}

#[test]
fn reveal_opens_the_project_folder() {
    let fx = Fx::new();

    fx.runner().reveal("p", NOW).unwrap();

    assert_eq!(fx.system.calls(), [format!("open: {}", fx.dir_text())]);
}

#[test]
fn the_project_file_opens_in_the_first_editor_that_is_set() {
    let cases = [
        ("config editor", ("Nova", "Fork", "Zed"), "Nova"),
        ("project editor", ("", "Fork", "Zed"), "Fork"),
        ("blank config editor", ("  ", "Fork", "Zed"), "Fork"),
        ("default editor", ("", "", "Zed"), "Zed"),
    ];
    for (name, (config_editor, project_editor, default_editor), expected) in cases {
        let mut file = project_file();
        if !project_editor.is_empty() {
            file["meta"] = json!({ "editor": project_editor });
        }
        let fx = Fx::with(Some(file), settings_with("", config_editor, default_editor));

        fx.runner().edit_file("p", NOW).unwrap();

        assert_eq!(
            fx.system.calls(),
            [format!("spawn: {}", open_with(expected, &fx.file_text()))],
            "{name}"
        );
    }
}

#[test]
fn the_project_file_opens_as_text_without_any_editor() {
    let fx = Fx::new();

    fx.runner().edit_file("p", NOW).unwrap();

    assert_eq!(fx.system.calls(), [format!("open: -t {}", fx.file_text())]);
}

#[test]
fn a_folder_without_the_project_file_cannot_be_edited() {
    let fx = Fx::with(None, Settings::default());

    let err = fx.runner().edit_file("p", NOW).unwrap_err();

    assert_eq!(err.kind(), ErrorKind::NotFound);
    assert_eq!(fx.last_opened_at(), None);
    assert!(fx.system.calls().is_empty());
}

#[test]
fn status_lists_the_states_and_hides_actions_named_by_a_current_state() {
    let cases = [
        (
            "up",
            vec![StateDisplay {
                label: "Site".into(),
                value: "Online".into(),
                color: Some("green".into()),
            }],
            vec![apps(3), scripts(0)],
        ),
        (
            "down",
            vec![StateDisplay {
                label: "Site".into(),
                value: "Offline".into(),
                color: None,
            }],
            vec![scripts(1)],
        ),
        (
            "degraded",
            vec![StateDisplay {
                label: "Site".into(),
                value: "degraded".into(),
                color: None,
            }],
            vec![],
        ),
    ];
    for (output, states, hidden) in cases {
        let fx = Fx::new();
        fx.system
            .reply(&format!("curl-demo {}", fx.dir_text()), output);

        let status = fx.runner().status("p").unwrap();

        assert_eq!(
            status,
            ProjectStatus {
                states,
                hidden,
                running: vec![],
            },
            "{output}"
        );
    }
}

#[test]
fn status_has_no_states_and_hides_nothing_when_the_provider_prints_nothing() {
    let fx = Fx::new();

    assert_eq!(fx.runner().status("p").unwrap(), ProjectStatus::default());
}

#[test]
fn status_lists_a_terminal_command_while_its_session_runs() {
    let fx = Fx::new();
    fx.runner().launch("p", apps(3), NOW).unwrap();
    assert_eq!(
        fx.runner().status("p").unwrap().running,
        Vec::<ActionSource>::new()
    );

    fx.record_session();
    fx.system.reply("ps -o tty= -p 4242", "ttys004\n");

    assert_eq!(fx.runner().status("p").unwrap().running, [apps(3)]);
}

#[test]
fn a_script_without_a_reply_in_the_recorder_reports_that_it_did_not_run() {
    let fx = Fx::new();

    let outcome = fx.runner().launch("p", scripts(1), NOW).unwrap();

    let Outcome::Ran { result } = outcome else {
        panic!("a script runs to its end: {outcome:?}");
    };
    assert_eq!(
        result.output,
        "Not run: `make deploy`. Set LAUNCHER_REAL_LAUNCH=1 to run it."
    );
}

#[test]
fn outcomes_and_results_serialize_for_the_page() {
    let result = ScriptResult {
        ok: false,
        exit_code: Some(2),
        timed_out: false,
        output: "boom".into(),
        duration_ms: 7,
    };
    assert_eq!(
        serde_json::to_value(Outcome::Ran { result }).unwrap(),
        json!({
            "kind": "ran",
            "result": {
                "ok": false,
                "exitCode": 2,
                "timedOut": false,
                "output": "boom",
                "durationMs": 7,
            },
        })
    );
    assert_eq!(
        serde_json::to_value(Outcome::Opened {
            how: Launched::NeedsPermission
        })
        .unwrap(),
        json!({ "kind": "opened", "how": "needsPermission" })
    );
    assert_eq!(
        serde_json::to_value(ProjectStatus {
            states: vec![],
            hidden: vec![scripts(1)],
            running: vec![apps(3)],
        })
        .unwrap(),
        json!({
            "states": [],
            "hidden": [{ "list": "scripts", "index": 1 }],
            "running": [{ "list": "apps", "index": 3 }],
        })
    );
}
