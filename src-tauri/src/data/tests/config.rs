//! The launch config through `Data`: resolving a project's config, the action, built-in, lock and dev edits,
//! their undo and redo, and the demo seed's configs.

use std::fs;
use std::path::PathBuf;

use serde_json::{Value, json};
use tempfile::TempDir;

use crate::data::{Changed, Data, MetaEdit};
use crate::error::ErrorKind;
use crate::projects::config::resolve::ClaudeSetup;
use crate::projects::config::{ActionDraft, ActionList, ActionSource, Builtin, DevMode, RunsAs};
use crate::projects::file::FILE_NAME;
use crate::projects::{KnownProject, Registry};
use crate::store::Settings;
use crate::test_support::{demo, temp_config_dir};

const PROJECTS_ONLY: Changed = Changed {
    settings: false,
    projects: true,
};

/// One project, `p`, in a projects folder that doubles as the home folder.
struct Fx {
    _config: TempDir,
    root: TempDir,
    data: Data,
    known: Vec<KnownProject>,
}

impl Fx {
    fn new(file: Option<Value>) -> Self {
        Self::with_settings(file, Settings::default())
    }

    fn with_settings(file: Option<Value>, settings: Settings) -> Self {
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
        let known = vec![KnownProject {
            id: "p".into(),
            path: dir.to_string_lossy().into_owned(),
            added_at: 1000,
            last_opened_at: None,
        }];
        data.reset(
            settings,
            Registry {
                projects: known.clone(),
            },
        )
        .unwrap();
        Self {
            _config: config,
            root,
            data,
            known,
        }
    }

    fn dir(&self) -> PathBuf {
        self.root.path().join("p")
    }

    fn raw(&self) -> String {
        fs::read_to_string(self.dir().join(FILE_NAME)).unwrap()
    }

    fn file(&self) -> Value {
        serde_json::from_str(&self.raw()).unwrap()
    }

    fn keys(&self) -> Vec<String> {
        self.file().as_object().unwrap().keys().cloned().collect()
    }

    fn write_raw(&self, text: &str) {
        fs::write(self.dir().join(FILE_NAME), text).unwrap();
    }

    fn has_file(&self) -> bool {
        self.dir().join(FILE_NAME).exists()
    }

    fn replace_settings(&self, settings: Settings) {
        self.data
            .reset(
                settings,
                Registry {
                    projects: self.known.clone(),
                },
            )
            .unwrap();
    }
}

fn settings(editor: &str, git_client: &str) -> Settings {
    Settings {
        default_editor: editor.into(),
        git_client: git_client.into(),
        ..Settings::default()
    }
}

fn base_file() -> Value {
    json!({
        "name": "Alpha",
        "meta": { "color": "Red" },
        "apps": ["editor", "terminal"],
        "scripts": [{ "label": "Build", "command": "make" }],
        "custom": { "keep": true }
    })
}

fn locked_file() -> Value {
    json!({
        "name": "Gamma",
        "meta": { "icon": "Leaf", "color": "Yellow", "tag": "Personal", "notes": ["a"] },
        "locked": true,
        "apps": ["editor"],
        "scripts": []
    })
}

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

fn draft(runs_as: RunsAs, label: &str) -> ActionDraft {
    ActionDraft {
        runs_as,
        label: label.into(),
        ..ActionDraft::default()
    }
}

fn app_draft(label: &str, app: &str) -> ActionDraft {
    ActionDraft {
        app: Some(app.into()),
        ..draft(RunsAs::App, label)
    }
}

fn undo_label(data: &Data) -> Option<String> {
    data.undo().unwrap().map(|r| r.label)
}

fn labels(actions: &[crate::projects::config::resolve::Action]) -> Vec<&str> {
    actions.iter().map(|a| a.label.as_str()).collect()
}

fn enabled_builtins(config: &crate::projects::config::ProjectConfig) -> Vec<(Builtin, bool)> {
    config
        .builtins
        .iter()
        .map(|b| (b.preset, b.enabled))
        .collect()
}

#[test]
fn the_config_resolves_apps_scripts_and_builtins_from_the_file() {
    let fx = Fx::with_settings(Some(base_file()), settings("Visual Studio Code", ""));

    let config = fx.data.project_config("p").unwrap();

    assert_eq!(labels(&config.apps), ["Edit Code", "Open Terminal"]);
    assert_eq!(config.apps[0].app.as_deref(), Some("Visual Studio Code"));
    assert_eq!(config.apps[0].edit.source, apps(0));
    assert_eq!(labels(&config.scripts), ["Build"]);
    assert_eq!(config.scripts[0].command.as_deref(), Some("make"));
    assert_eq!(config.scripts[0].edit.source, scripts(0));
    assert_eq!(
        enabled_builtins(&config),
        [
            (Builtin::Editor, true),
            (Builtin::Terminal, true),
            (Builtin::Git, false),
            (Builtin::Browser, false),
            (Builtin::RepoBrowser, false),
            (Builtin::Claude, false),
        ]
    );
    assert!(!config.locked);
    assert_eq!(config.dev, DevMode::None);
}

#[test]
fn the_editor_follows_the_default_editor_setting_on_the_next_call() {
    let fx = Fx::with_settings(Some(base_file()), settings("Visual Studio Code", ""));
    assert_eq!(
        fx.data.project_config("p").unwrap().apps[0].app.as_deref(),
        Some("Visual Studio Code")
    );

    fx.replace_settings(settings("Zed", ""));

    assert_eq!(
        fx.data.project_config("p").unwrap().apps[0].app.as_deref(),
        Some("Zed")
    );
}

#[test]
fn an_editor_without_a_default_editor_is_unavailable_and_not_among_the_apps() {
    let fx = Fx::new(Some(base_file()));

    let config = fx.data.project_config("p").unwrap();

    assert_eq!(labels(&config.apps), ["Open Terminal"]);
    assert_eq!(
        config.builtins[0].unavailable_reason.as_deref(),
        Some("Choose a default editor in Settings")
    );
}

#[test]
fn the_git_repo_flag_and_the_git_app_follow_a_git_folder() {
    let file = json!({ "apps": ["git"] });
    let fx = Fx::with_settings(Some(file), settings("", "GitHub Desktop"));

    let without = fx.data.project_config("p").unwrap();
    assert!(!without.is_git_repo);
    assert!(without.apps.is_empty());

    fs::create_dir(fx.dir().join(".git")).unwrap();

    let with = fx.data.project_config("p").unwrap();
    assert!(with.is_git_repo);
    assert_eq!(labels(&with.apps), ["Open Git Client"]);
    assert_eq!(with.apps[0].app.as_deref(), Some("GitHub Desktop"));
}

#[test]
fn the_claude_setup_follows_a_claude_folder() {
    let file = json!({ "env": { "CLAUDE_CONFIG_DIR": "~/.claude-demo" } });
    let fx = Fx::new(Some(file));
    assert_eq!(fx.data.project_config("p").unwrap().claude, None);

    fs::create_dir(fx.dir().join(".claude")).unwrap();

    let home = fx.root.path().to_string_lossy().into_owned();
    assert_eq!(
        fx.data.project_config("p").unwrap().claude,
        Some(ClaudeSetup {
            config_dir: Some(format!("{home}/.claude-demo"))
        })
    );
}

#[test]
fn a_config_needs_a_known_project_and_a_readable_file() {
    let missing = Fx::new(None);
    assert_eq!(
        missing.data.project_config("p").unwrap_err().kind(),
        ErrorKind::NotFound
    );
    assert_eq!(
        missing.data.project_config("nope").unwrap_err().kind(),
        ErrorKind::NotFound
    );

    let broken = Fx::new(Some(json!({})));
    broken.write_raw("{ \"apps\": ");
    broken.data.reload();
    assert_eq!(
        broken.data.project_config("p").unwrap_err().kind(),
        ErrorKind::InvalidInput
    );
}

#[test]
fn a_new_action_is_appended_with_its_shortcut_in_file_notation() {
    let fx = Fx::new(Some(base_file()));
    let draft = ActionDraft {
        shortcut: Some("Mod+Shift+D".into()),
        ..app_draft("Docs", "Safari")
    };

    assert_eq!(
        fx.data.save_action("p", None, &draft, None).unwrap(),
        PROJECTS_ONLY
    );

    let file = fx.file();
    assert_eq!(
        file["apps"],
        json!([
            "editor",
            "terminal",
            { "label": "Docs", "app": "Safari", "shortcut": "cmd+shift+d" }
        ])
    );
    assert_eq!(file["scripts"], base_file()["scripts"]);
    assert_eq!(file["custom"], json!({ "keep": true }));
    assert_eq!(fx.keys(), ["name", "meta", "apps", "scripts", "custom"]);
}

#[test]
fn a_new_script_goes_to_the_scripts_list() {
    let fx = Fx::new(Some(base_file()));
    let draft = ActionDraft {
        command: Some("make test".into()),
        ..draft(RunsAs::Script, "Test")
    };

    fx.data.save_action("p", None, &draft, None).unwrap();

    assert_eq!(
        fx.file()["scripts"],
        json!([{ "label": "Build", "command": "make" }, { "label": "Test", "command": "make test" }])
    );
    assert_eq!(fx.file()["apps"], base_file()["apps"]);
}

#[test]
fn editing_a_builtin_writes_a_preset_entry_in_its_place() {
    let fx = Fx::new(Some(base_file()));
    let draft = ActionDraft {
        label: "Shell".into(),
        shortcut: Some("Mod+Shift+T".into()),
        ..ActionDraft::default()
    };

    fx.data
        .save_action("p", Some(apps(1)), &draft, Some(Builtin::Terminal))
        .unwrap();

    assert_eq!(
        fx.file()["apps"],
        json!([
            "editor",
            { "preset": "terminal", "label": "Shell", "shortcut": "cmd+shift+t" }
        ])
    );
}

#[test]
fn changing_a_script_into_an_app_moves_it_between_the_lists_as_one_undo_step() {
    let fx = Fx::new(Some(base_file()));

    fx.data
        .save_action("p", Some(scripts(0)), &app_draft("Build", "Safari"), None)
        .unwrap();

    let file = fx.file();
    assert_eq!(file["scripts"], json!([]));
    assert_eq!(
        file["apps"],
        json!(["editor", "terminal", { "label": "Build", "app": "Safari" }])
    );
    assert_eq!(fx.keys(), ["name", "meta", "apps", "scripts", "custom"]);

    let undone = fx.data.undo().unwrap().unwrap();
    assert_eq!(undone.label, "Edit Action");
    assert_eq!(fx.file(), base_file());
    assert_eq!(fx.keys(), ["name", "meta", "apps", "scripts", "custom"]);
    assert_eq!(undo_label(&fx.data), None);

    let redone = fx.data.redo().unwrap().unwrap();
    assert_eq!(redone.label, "Edit Action");
    assert_eq!(fx.file()["scripts"], json!([]));
    assert_eq!(
        fx.file()["apps"][2],
        json!({ "label": "Build", "app": "Safari" })
    );
}

#[test]
fn each_edit_is_one_undo_step_with_its_label_and_undoes_back_to_the_start() {
    let fx = Fx::new(Some(base_file()));
    let data = &fx.data;
    data.save_action("p", None, &app_draft("Docs", "Safari"), None)
        .unwrap();
    data.save_action("p", Some(apps(2)), &app_draft("Manual", "Safari"), None)
        .unwrap();
    data.move_action("p", apps(2), 0).unwrap();
    data.remove_action("p", apps(0)).unwrap();
    data.set_builtin_enabled("p", Builtin::Git, true).unwrap();
    data.set_builtin_enabled("p", Builtin::Git, false).unwrap();
    data.set_project_locked("p", true).unwrap();
    data.set_project_locked("p", false).unwrap();
    data.set_dev_mode("p", DevMode::Ddev).unwrap();

    let mut undone = Vec::new();
    while let Some(label) = undo_label(data) {
        undone.push(label);
    }

    assert_eq!(
        undone,
        [
            "Change Dev Environment",
            "Unlock Project Details",
            "Lock Project Details",
            "Remove Built-in App",
            "Add Built-in App",
            "Remove Action",
            "Move Action",
            "Edit Action",
            "New Action",
        ]
    );
    assert_eq!(fx.file(), base_file());
    assert_eq!(fx.keys(), ["name", "meta", "apps", "scripts", "custom"]);
}

#[test]
fn moving_an_action_reorders_its_list_and_past_the_end_means_last() {
    let fx = Fx::new(Some(base_file()));

    fx.data.move_action("p", apps(0), 99).unwrap();
    assert_eq!(fx.file()["apps"], json!(["terminal", "editor"]));

    fx.data.move_action("p", apps(1), 0).unwrap();
    assert_eq!(fx.file()["apps"], json!(["editor", "terminal"]));
}

#[test]
fn removing_an_action_drops_it_from_its_list_only() {
    let fx = Fx::new(Some(base_file()));

    fx.data.remove_action("p", scripts(0)).unwrap();

    assert_eq!(fx.file()["scripts"], json!([]));
    assert_eq!(fx.file()["apps"], base_file()["apps"]);
}

#[test]
fn a_stale_index_is_refused_and_leaves_the_file_alone() {
    let fx = Fx::new(Some(base_file()));
    let before = fx.raw();

    let results = [
        ("remove", fx.data.remove_action("p", apps(9))),
        ("move", fx.data.move_action("p", scripts(1), 0)),
        (
            "edit",
            fx.data
                .save_action("p", Some(apps(9)), &app_draft("Docs", "Safari"), None),
        ),
        (
            "preset",
            fx.data.save_action(
                "p",
                Some(apps(0)),
                &draft(RunsAs::App, "x"),
                Some(Builtin::Git),
            ),
        ),
    ];

    for (name, result) in results {
        assert_eq!(
            result.unwrap_err().kind(),
            ErrorKind::InvalidInput,
            "{name}"
        );
    }
    assert_eq!(fx.raw(), before);
    assert_eq!(undo_label(&fx.data), None);
}

#[test]
fn enabling_a_builtin_appends_its_name_and_disabling_removes_every_entry_of_it() {
    let fx = Fx::new(Some(json!({ "apps": ["editor"] })));

    fx.data
        .set_builtin_enabled("p", Builtin::Git, true)
        .unwrap();
    assert_eq!(fx.file()["apps"], json!(["editor", "git"]));

    let fx = Fx::new(Some(json!({
        "apps": ["editor", { "preset": "git", "label": "Repo" }, "git"]
    })));

    fx.data
        .set_builtin_enabled("p", Builtin::Git, false)
        .unwrap();
    assert_eq!(fx.file()["apps"], json!(["editor"]));

    fx.data.undo().unwrap();
    assert_eq!(
        fx.file()["apps"],
        json!(["editor", { "preset": "git", "label": "Repo" }, "git"])
    );
}

#[test]
fn the_dev_mode_is_written_as_its_flag_and_none_removes_the_key() {
    let fx = Fx::new(Some(base_file()));

    for (mode, expected) in [
        (DevMode::Ddev, json!({ "ddev": true })),
        (DevMode::WpEnv, json!({ "wpEnv": true })),
        (DevMode::Docker, json!({ "docker": true })),
    ] {
        fx.data.set_dev_mode("p", mode).unwrap();
        assert_eq!(fx.file()["dev"], expected, "{mode:?}");
        assert_eq!(fx.data.project_config("p").unwrap().dev, mode, "{mode:?}");
    }
    assert_eq!(
        fx.keys(),
        ["name", "meta", "apps", "scripts", "custom", "dev"]
    );

    fx.data.set_dev_mode("p", DevMode::None).unwrap();

    assert_eq!(fx.file(), base_file());
}

#[test]
fn locking_writes_true_and_unlocking_removes_the_key() {
    let fx = Fx::new(Some(base_file()));

    fx.data.set_project_locked("p", true).unwrap();
    assert_eq!(fx.file()["locked"], json!(true));
    assert!(fx.data.project_config("p").unwrap().locked);

    fx.data.set_project_locked("p", false).unwrap();
    assert_eq!(fx.file(), base_file());
    assert!(!fx.data.project_config("p").unwrap().locked);
}

#[test]
fn a_change_that_changes_nothing_records_no_undo_step() {
    let fx = Fx::new(Some(base_file()));

    let results = [
        fx.data.set_builtin_enabled("p", Builtin::Editor, true),
        fx.data.set_builtin_enabled("p", Builtin::Git, false),
        fx.data.set_dev_mode("p", DevMode::None),
        fx.data.move_action("p", apps(0), 0),
        fx.data.set_project_locked("p", false),
    ];
    for result in results {
        assert_eq!(result.unwrap(), Changed::default());
    }
    assert_eq!(undo_label(&fx.data), None);

    let locked = Fx::new(Some(
        json!({ "name": "A", "locked": true, "dev": { "ddev": true } }),
    ));
    assert_eq!(
        locked.data.set_project_locked("p", true).unwrap(),
        Changed::default()
    );
    assert_eq!(undo_label(&locked.data), None);

    let dev = Fx::new(Some(json!({ "name": "A", "dev": { "ddev": true } })));
    assert_eq!(
        dev.data.set_dev_mode("p", DevMode::Ddev).unwrap(),
        Changed::default()
    );
    assert_eq!(undo_label(&dev.data), None);
}

#[test]
fn a_locked_project_refuses_every_edit_of_its_details_and_stays_as_it_was() {
    let fx = Fx::new(Some(locked_file()));
    let before = fx.raw();
    let data = &fx.data;

    let results = [
        (
            "save",
            data.save_action("p", None, &app_draft("Docs", "Safari"), None),
        ),
        ("remove", data.remove_action("p", apps(0))),
        ("move", data.move_action("p", apps(0), 0)),
        (
            "builtin on",
            data.set_builtin_enabled("p", Builtin::Git, true),
        ),
        (
            "builtin off",
            data.set_builtin_enabled("p", Builtin::Editor, false),
        ),
        ("dev", data.set_dev_mode("p", DevMode::Ddev)),
        ("name", data.set_project_name("p", "Renamed")),
        (
            "icon",
            data.set_project_meta("p", MetaEdit::Icon(Some("Bolt".into()))),
        ),
        (
            "color",
            data.set_project_meta("p", MetaEdit::Color(Some("Red".into()))),
        ),
        (
            "tag",
            data.set_project_meta("p", MetaEdit::Tag(Some("Client".into()))),
        ),
        (
            "notes",
            data.set_project_meta("p", MetaEdit::Notes(vec!["b".into()])),
        ),
    ];

    for (name, result) in results {
        assert_eq!(
            result.unwrap_err().kind(),
            ErrorKind::InvalidInput,
            "{name}"
        );
    }
    assert_eq!(fx.raw(), before);
    assert_eq!(undo_label(data), None);
}

#[test]
fn a_locked_project_can_still_be_starred_archived_and_unlocked() {
    let fx = Fx::new(Some(locked_file()));

    fx.data
        .set_project_meta("p", MetaEdit::Starred(true))
        .unwrap();
    assert_eq!(fx.file()["meta"]["starred"], json!(true));
    fx.data
        .set_project_meta("p", MetaEdit::Archived(true))
        .unwrap();
    assert_eq!(fx.file()["meta"]["archived"], json!(true));
    assert_eq!(fx.file()["meta"].get("starred"), None);
    assert_eq!(fx.file()["locked"], json!(true));

    fx.data.set_project_locked("p", false).unwrap();
    assert_eq!(fx.file().get("locked"), None);

    fx.data.set_project_name("p", "Renamed").unwrap();
    fx.data.set_dev_mode("p", DevMode::Docker).unwrap();
    assert_eq!(fx.file()["name"], json!("Renamed"));
    assert_eq!(fx.file()["dev"], json!({ "docker": true }));
}

#[test]
fn undoing_an_edit_works_after_the_project_was_locked() {
    let fx = Fx::new(Some(base_file()));
    fx.data
        .save_action("p", None, &app_draft("Docs", "Safari"), None)
        .unwrap();
    let mut locked = fx.file();
    locked["locked"] = json!(true);
    fx.write_raw(&serde_json::to_string_pretty(&locked).unwrap());
    fx.data.reload();

    let undone = fx.data.undo().unwrap().unwrap();

    assert_eq!(undone.label, "New Action");
    assert_eq!(fx.file()["apps"], base_file()["apps"]);
    assert_eq!(fx.file()["locked"], json!(true));
}

#[test]
fn config_edits_on_a_missing_file_fail_and_do_not_create_it() {
    let fx = Fx::new(None);
    let data = &fx.data;

    let results = [
        (
            "save",
            data.save_action("p", None, &app_draft("Docs", "Safari"), None),
        ),
        ("remove", data.remove_action("p", apps(0))),
        ("move", data.move_action("p", apps(0), 0)),
        ("builtin", data.set_builtin_enabled("p", Builtin::Git, true)),
        ("dev", data.set_dev_mode("p", DevMode::Ddev)),
        ("lock", data.set_project_locked("p", true)),
    ];

    for (name, result) in results {
        assert_eq!(result.unwrap_err().kind(), ErrorKind::NotFound, "{name}");
    }
    assert!(!fx.has_file());
}

#[test]
fn config_edits_on_an_unknown_project_are_not_found() {
    let fx = Fx::new(Some(base_file()));

    assert_eq!(
        fx.data
            .set_dev_mode("nope", DevMode::Ddev)
            .unwrap_err()
            .kind(),
        ErrorKind::NotFound
    );
}

const NOW: i64 = 1_800_000_000_000;

fn demo_data(config: &TempDir) -> Data {
    let seed = demo::write(config.path(), NOW).unwrap();
    let data = Data::open_with_home(config.path(), Some(PathBuf::from("/home/demo")));
    data.reset(seed.settings, seed.registry).unwrap();
    data
}

#[test]
fn the_demo_acme_website_lists_its_apps_without_the_unavailable_git_client() {
    let config = temp_config_dir();
    let data = demo_data(&config);

    let acme = data.project_config("demo00000001").unwrap();

    assert_eq!(
        labels(&acme.apps),
        ["Edit Code", "Shell", "Open in Browser", "Open Repository"]
    );
    assert_eq!(acme.apps[0].app.as_deref(), Some("Visual Studio Code"));
    assert_eq!(acme.apps[1].shortcut.as_deref(), Some("Mod+Shift+T"));
    assert_eq!(labels(&acme.scripts), ["Build", "Lint", "Export Database"]);
    assert_eq!(acme.dev, DevMode::Ddev);
    assert!(!acme.is_git_repo);
    let git = acme
        .builtins
        .iter()
        .find(|b| b.preset == Builtin::Git)
        .unwrap();
    assert!(git.enabled);
    assert_eq!(
        git.unavailable_reason.as_deref(),
        Some("Not a git repository")
    );
}

#[test]
fn the_demo_orbit_api_resolves_env_states_and_the_claude_app() {
    let config = temp_config_dir();
    let data = demo_data(&config);

    let orbit = data.project_config("demo00000002").unwrap();

    assert_eq!(
        labels(&orbit.apps),
        [
            "Edit Code",
            "Open Terminal",
            "Claude Code",
            "API Docs",
            "Tail Logs"
        ]
    );
    assert_eq!(orbit.apps[3].runs_as, RunsAs::Url);
    assert_eq!(orbit.apps[4].hidden_states, ["paused"]);
    let env: Vec<(&str, &str)> = orbit
        .env
        .iter()
        .map(|v| (v.name.as_str(), v.value.as_str()))
        .collect();
    assert_eq!(
        env,
        [
            ("APP_ENV", "local"),
            ("CLAUDE_CONFIG_DIR", "/home/demo/.claude-demo")
        ]
    );
    assert_eq!(orbit.dev, DevMode::Docker);
    assert_eq!(orbit.states.len(), 1);
    let values: Vec<&str> = orbit.states[0]
        .values
        .iter()
        .map(|v| v.key.as_str())
        .collect();
    assert_eq!(values, ["running", "paused"]);
    assert_eq!(orbit.state_providers.len(), 1);
    assert_eq!(orbit.claude, None);
}

#[test]
fn the_demo_orbit_api_gets_its_claude_setup_once_it_has_a_claude_folder() {
    let config = temp_config_dir();
    let data = demo_data(&config);
    let dir = config.path().join(demo::DEMO_FOLDER).join("orbit-api");
    fs::create_dir(dir.join(".claude")).unwrap();

    let orbit = data.project_config("demo00000002").unwrap();

    assert_eq!(
        orbit.claude,
        Some(ClaudeSetup {
            config_dir: Some("/home/demo/.claude-demo".into())
        })
    );
}

#[test]
fn the_demo_garden_planner_is_locked_and_refuses_edits() {
    let config = temp_config_dir();
    let data = demo_data(&config);

    assert!(data.project_config("demo00000004").unwrap().locked);
    assert_eq!(
        data.set_dev_mode("demo00000004", DevMode::Ddev)
            .unwrap_err()
            .kind(),
        ErrorKind::InvalidInput
    );
}
