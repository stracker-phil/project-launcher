//! The launch config of a project file, resolved for the page and the launchers: built-ins expanded with the
//! settings, placeholders filled in, entries that don't fit skipped. Each action carries what the action
//! editor needs to change it. Pure: the folder facts come in through `Context`.

use std::path::Path;

use serde::Serialize;
use serde_json::{Map, Value};
use ts_rs::TS;

use super::builtins::{Links, builtin_apps, runs_as, target};
use super::model::{
    ActionDraft, ActionList, ActionSource, Builtin, DatabaseSetting, DevMode, RunsAs, page_key,
};
use super::vars::Vars;
use crate::projects::file::FileObject;
use crate::projects::view::text;
use crate::store::Settings;

const CUSTOM_APP_ICON: &str = "Window";
const SCRIPT_ICON: &str = "Terminal";

/// The facts about a project's folder the resolver needs.
pub struct Context<'a> {
    pub dir: &'a Path,
    pub home: Option<&'a Path>,
    pub settings: &'a Settings,
    pub is_git_repo: bool,
    /// The folder has a `.claude` directory.
    pub uses_claude: bool,
}

#[derive(Serialize, TS, Debug, Clone, PartialEq)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct ProjectConfig {
    /// The `apps` entries that can run here, in file order.
    pub apps: Vec<Action>,
    pub scripts: Vec<Action>,
    /// Every built-in, enabled or not, for the action editor's menu.
    pub builtins: Vec<BuiltinApp>,
    pub env: Vec<EnvVar>,
    pub state_providers: Vec<StateProvider>,
    pub states: Vec<StateDef>,
    pub dev: DevMode,
    pub database: DatabaseSetting,
    /// Present when the folder has a `.claude` directory.
    pub claude: Option<ClaudeSetup>,
    /// The details are read-only in the app.
    pub locked: bool,
    pub is_git_repo: bool,
}

/// An app, URL, terminal command or script, with its placeholders filled in.
#[derive(Serialize, TS, Debug, Clone, PartialEq)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct Action {
    pub label: String,
    pub icon: String,
    pub color: Option<String>,
    /// In the page's notation (`Mod+O`).
    pub shortcut: Option<String>,
    pub runs_as: RunsAs,
    pub app: Option<String>,
    pub args: Option<String>,
    pub url: Option<String>,
    /// For a terminal, empty opens a shell in the project folder.
    pub command: Option<String>,
    /// The state keys that hide it.
    pub hidden_states: Vec<String>,
    pub edit: ActionEdit,
}

/// What the action editor starts from.
#[derive(Serialize, TS, Debug, Clone, PartialEq)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct ActionEdit {
    pub source: ActionSource,
    /// As written; for a built-in only the fields that override a default.
    pub draft: ActionDraft,
    pub preset: Option<Builtin>,
    /// Built-ins only: what each field falls back to.
    pub defaults: Option<ActionDraft>,
}

#[derive(Serialize, TS, Debug, Clone, PartialEq)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct BuiltinApp {
    pub preset: Builtin,
    pub label: String,
    pub icon: String,
    pub enabled: bool,
    /// Why it can't run in this project.
    pub unavailable_reason: Option<String>,
}

#[derive(Serialize, TS, Debug, Clone, PartialEq)]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct EnvVar {
    pub name: String,
    pub value: String,
}

/// A command whose output states read.
#[derive(Serialize, TS, Debug, Clone, PartialEq)]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct StateProvider {
    pub name: String,
    pub command: String,
}

/// A state shown on the project: `source` names a provider, or `provider:.path` a value in its JSON output.
#[derive(Serialize, TS, Debug, Clone, PartialEq)]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct StateDef {
    pub name: String,
    pub source: String,
    pub label: String,
    pub values: Vec<StateValue>,
}

/// How one output of a state shows; `key` is what `hiddenStates` refers to.
#[derive(Serialize, TS, Debug, Clone, PartialEq)]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct StateValue {
    pub key: String,
    pub value: String,
    pub label: String,
    pub color: Option<String>,
}

#[derive(Serialize, TS, Debug, Clone, PartialEq)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct ClaudeSetup {
    /// `CLAUDE_CONFIG_DIR` from the env; none uses the default profile.
    pub config_dir: Option<String>,
}

pub fn resolve(file: &FileObject, cx: &Context) -> ProjectConfig {
    let meta = file.get("meta");
    let meta_text = |key: &str| text(meta.and_then(|m| m.get(key)));
    let url = meta_text("url");
    let vars = Vars {
        dir: cx.dir,
        url: url.as_deref(),
        home: cx.home,
    };
    let links = Links {
        url: url.clone(),
        repo_url: meta_text("repoUrl"),
        editor: meta_text("editor"),
    };
    let env = env(file.get("env"), &vars);
    let apps_entries = entries(file.get("apps"));
    ProjectConfig {
        apps: apps_entries
            .iter()
            .enumerate()
            .filter_map(|(index, entry)| app(entry, index, cx, &links, &vars))
            .collect(),
        scripts: entries(file.get("scripts"))
            .iter()
            .enumerate()
            .filter_map(|(index, entry)| script(entry, index, &vars))
            .collect(),
        builtins: builtin_apps(apps_entries, cx, &links),
        claude: claude_setup(&env, cx),
        env,
        state_providers: object(file.get("stateProviders"))
            .filter_map(|(name, command)| {
                Some(StateProvider {
                    name: name.clone(),
                    command: vars.fill(command.as_str()?),
                })
            })
            .collect(),
        states: object(file.get("states"))
            .filter_map(|(name, state)| state_def(name, state, &vars))
            .collect(),
        dev: DevMode::of(file.get("dev")),
        database: DatabaseSetting::of(file.get("database")),
        locked: file.get("locked") == Some(&Value::Bool(true)),
        is_git_repo: cx.is_git_repo,
    }
}

fn app(entry: &Value, index: usize, cx: &Context, links: &Links, vars: &Vars) -> Option<Action> {
    let source = ActionSource {
        list: ActionList::Apps,
        index,
    };
    match Builtin::of(entry) {
        Some(preset) => builtin_app(entry, preset, source, cx, links, vars),
        None => custom_app(entry, source, vars),
    }
}

/// A built-in with the entry's overrides; skipped where it can't run.
fn builtin_app(
    entry: &Value,
    preset: Builtin,
    source: ActionSource,
    cx: &Context,
    links: &Links,
    vars: &Vars,
) -> Option<Action> {
    let target = target(preset, cx, links).ok()?;
    let own = |key: &str| non_empty(entry, key);
    let defaults = ActionDraft {
        runs_as: runs_as(target.app.is_some(), target.url.is_some()),
        label: preset.label().into(),
        app: target.app.clone(),
        url: target.url.clone(),
        command: target.command.clone(),
        icon: Some(preset.icon().into()),
        shortcut: page_key(preset.shortcut()),
        ..ActionDraft::default()
    };
    Some(Action {
        label: own("label").unwrap_or(preset.label()).into(),
        icon: own("icon").unwrap_or(preset.icon()).into(),
        color: own("color").map(str::to_string),
        shortcut: page_key(own("shortcut").unwrap_or(preset.shortcut())),
        runs_as: defaults.runs_as,
        app: own("app").map(str::to_string).or(target.app),
        args: own("args").map(|s| vars.fill(s)),
        url: own("url").map(|s| vars.fill(s)).or(target.url),
        command: own("command").map(|s| vars.fill(s)).or(target.command),
        hidden_states: hidden_states(entry),
        edit: ActionEdit {
            source,
            draft: ActionDraft::of_entry(entry.is_object().then_some(entry), defaults.runs_as),
            preset: Some(preset),
            defaults: Some(defaults),
        },
    })
}

/// An entry of its own; it needs a label.
fn custom_app(entry: &Value, source: ActionSource, vars: &Vars) -> Option<Action> {
    let label = entry.get("label")?.as_str()?;
    let own = |key: &str| non_empty(entry, key);
    let runs_as = runs_as(own("app").is_some(), own("url").is_some());
    Some(Action {
        label: label.into(),
        icon: own("icon").unwrap_or(CUSTOM_APP_ICON).into(),
        color: own("color").map(str::to_string),
        shortcut: own("shortcut").and_then(page_key),
        runs_as,
        app: own("app").map(str::to_string),
        args: own("args").map(|s| vars.fill(s)),
        url: own("url").map(|s| vars.fill(s)),
        // An empty command is kept: it opens a shell in the project folder.
        command: entry
            .get("command")
            .and_then(Value::as_str)
            .map(|s| vars.fill(s)),
        hidden_states: hidden_states(entry),
        edit: ActionEdit {
            source,
            draft: ActionDraft::of_entry(Some(entry), runs_as),
            preset: None,
            defaults: None,
        },
    })
}

fn script(entry: &Value, index: usize, vars: &Vars) -> Option<Action> {
    let label = entry.get("label")?.as_str()?;
    let command = entry.get("command")?.as_str()?;
    let own = |key: &str| non_empty(entry, key);
    Some(Action {
        label: label.into(),
        icon: own("icon").unwrap_or(SCRIPT_ICON).into(),
        color: own("color").map(str::to_string),
        shortcut: own("shortcut").and_then(page_key),
        runs_as: RunsAs::Script,
        app: None,
        args: None,
        url: None,
        command: Some(vars.fill(command)),
        hidden_states: hidden_states(entry),
        edit: ActionEdit {
            source: ActionSource {
                list: ActionList::Scripts,
                index,
            },
            draft: ActionDraft::of_entry(Some(entry), RunsAs::Script),
            preset: None,
            defaults: None,
        },
    })
}

/// The entry's string field, when it has text.
fn non_empty<'a>(entry: &'a Value, key: &str) -> Option<&'a str> {
    entry
        .get(key)
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
}

fn hidden_states(entry: &Value) -> Vec<String> {
    entries(entry.get("hiddenStates"))
        .iter()
        .filter_map(|key| key.as_str().map(str::to_string))
        .collect()
}

fn claude_setup(env: &[EnvVar], cx: &Context) -> Option<ClaudeSetup> {
    cx.uses_claude.then(|| ClaudeSetup {
        config_dir: env
            .iter()
            .find(|v| v.name == "CLAUDE_CONFIG_DIR")
            .map(|v| v.value.clone()),
    })
}

/// Strings as written, numbers and booleans as text; other values are skipped.
fn env(value: Option<&Value>, vars: &Vars) -> Vec<EnvVar> {
    object(value)
        .filter_map(|(name, value)| {
            let raw = match value {
                Value::String(s) => s.clone(),
                Value::Number(n) => n.to_string(),
                Value::Bool(b) => b.to_string(),
                _ => return None,
            };
            Some(EnvVar {
                name: name.clone(),
                value: vars.fill(&raw),
            })
        })
        .collect()
}

/// A state needs a `source`; its label falls back to its name, and values without `value` and `label` are
/// skipped.
fn state_def(name: &str, state: &Value, vars: &Vars) -> Option<StateDef> {
    let source = state.get("source")?.as_str()?;
    let values = object(state.get("values"))
        .filter_map(|(key, v)| {
            Some(StateValue {
                key: key.clone(),
                value: v.get("value")?.as_str()?.into(),
                label: v.get("label")?.as_str()?.into(),
                color: text(v.get("color")),
            })
        })
        .collect();
    Some(StateDef {
        name: name.into(),
        source: vars.fill(source),
        label: text(state.get("label")).unwrap_or_else(|| name.into()),
        values,
    })
}

/// The array's items; anything else has none.
pub fn entries(value: Option<&Value>) -> &[Value] {
    match value {
        Some(Value::Array(items)) => items,
        _ => &[],
    }
}

fn object(value: Option<&Value>) -> impl Iterator<Item = (&String, &Value)> {
    value
        .and_then(Value::as_object)
        .into_iter()
        .flat_map(Map::iter)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn resolve_in(
        file: Value,
        settings: &Settings,
        is_git_repo: bool,
        uses_claude: bool,
    ) -> ProjectConfig {
        let cx = Context {
            dir: Path::new("/work/northwind"),
            home: Some(Path::new("/Users/demo")),
            settings,
            is_git_repo,
            uses_claude,
        };
        resolve(file.as_object().unwrap(), &cx)
    }

    fn configured() -> Settings {
        Settings {
            default_editor: "Zed".into(),
            git_client: "Fork".into(),
            ..Settings::default()
        }
    }

    fn resolved(file: Value) -> ProjectConfig {
        resolve_in(file, &Settings::default(), false, false)
    }

    fn resolved_configured(file: Value) -> ProjectConfig {
        resolve_in(file, &configured(), true, false)
    }

    fn reasons(config: &ProjectConfig) -> Vec<(Builtin, Option<&str>)> {
        config
            .builtins
            .iter()
            .map(|b| (b.preset, b.unavailable_reason.as_deref()))
            .collect()
    }

    #[test]
    fn builtins_expand_with_their_labels_icons_shortcuts_and_targets() {
        let config = resolved_configured(json!({
            "meta": { "url": "https://example.com", "repoUrl": "https://example.com/northwind.git" },
            "apps": ["editor", "terminal", "git", "browser", "repoBrowser", "claude"],
        }));
        let rows: Vec<_> = config
            .apps
            .iter()
            .map(|a| {
                (
                    a.label.as_str(),
                    a.icon.as_str(),
                    a.shortcut.as_deref(),
                    a.runs_as,
                    a.app.as_deref(),
                    a.url.as_deref(),
                    a.command.as_deref(),
                )
            })
            .collect();
        assert_eq!(
            rows,
            vec![
                (
                    "Edit Code",
                    "Code",
                    Some("Mod+O"),
                    RunsAs::App,
                    Some("Zed"),
                    None,
                    None
                ),
                (
                    "Open Terminal",
                    "Terminal",
                    Some("Mod+T"),
                    RunsAs::Terminal,
                    None,
                    None,
                    Some("")
                ),
                (
                    "Open Git Client",
                    "CheckList",
                    Some("Mod+G"),
                    RunsAs::App,
                    Some("Fork"),
                    None,
                    None
                ),
                (
                    "Open in Browser",
                    "Globe",
                    Some("Mod+B"),
                    RunsAs::Url,
                    None,
                    Some("https://example.com"),
                    None
                ),
                (
                    "Open Repository",
                    "Link",
                    Some("Mod+Shift+R"),
                    RunsAs::Url,
                    None,
                    Some("https://example.com/northwind.git"),
                    None
                ),
                (
                    "Claude Code",
                    "Stars",
                    Some("Mod+Shift+K"),
                    RunsAs::Terminal,
                    None,
                    None,
                    Some("claude")
                ),
            ]
        );
    }

    #[test]
    fn the_project_editor_wins_over_the_default_editor() {
        let config =
            resolved_configured(json!({ "meta": { "editor": "Nova" }, "apps": ["editor"] }));
        assert_eq!(config.apps[0].app.as_deref(), Some("Nova"));
    }

    #[test]
    fn the_project_editor_makes_the_editor_available_without_a_default() {
        let config = resolved(json!({ "meta": { "editor": "Nova" }, "apps": ["editor"] }));
        assert_eq!(config.apps[0].app.as_deref(), Some("Nova"));
        assert_eq!(config.builtins[0].unavailable_reason, None);
    }

    #[test]
    fn unavailable_builtins_are_skipped_from_apps_and_say_why() {
        let config = resolve_in(
            json!({ "apps": ["editor", "terminal", "git", "browser", "repoBrowser", "claude"] }),
            &Settings::default(),
            false,
            false,
        );
        let labels: Vec<&str> = config.apps.iter().map(|a| a.label.as_str()).collect();
        assert_eq!(labels, ["Open Terminal", "Claude Code"]);
        assert_eq!(
            reasons(&config),
            vec![
                (Builtin::Editor, Some("Choose a default editor in Settings")),
                (Builtin::Terminal, None),
                (Builtin::Git, Some("Not a git repository")),
                (Builtin::Browser, Some("Add a website URL first")),
                (Builtin::RepoBrowser, Some("Add a repository URL first")),
                (Builtin::Claude, None),
            ]
        );
        assert!(config.builtins.iter().all(|b| b.enabled));
    }

    #[test]
    fn git_in_a_repository_needs_a_git_client() {
        let config = resolve_in(
            json!({ "apps": ["git"] }),
            &Settings::default(),
            true,
            false,
        );
        assert_eq!(
            config.builtins[2].unavailable_reason.as_deref(),
            Some("Choose a git client in Settings")
        );
        assert!(config.apps.is_empty());
        assert!(config.is_git_repo);
    }

    #[test]
    fn blank_settings_count_as_unset() {
        let settings = Settings {
            default_editor: "  ".into(),
            git_client: " ".into(),
            ..Settings::default()
        };
        let config = resolve_in(json!({ "apps": ["editor", "git"] }), &settings, true, false);
        assert!(config.apps.is_empty());
    }

    #[test]
    fn preset_overrides_win_and_get_their_placeholders_filled() {
        let config = resolved(json!({
            "meta": { "url": "https://example.com" },
            "apps": [
                {
                    "preset": "terminal",
                    "label": "Shell",
                    "icon": "Bolt",
                    "color": "teal",
                    "shortcut": "cmd+e",
                    "command": "cd ${dir} && ls ~/x",
                },
                { "preset": "browser", "url": "${url}/admin", "args": "${dir}" },
            ],
        }));
        let shell = &config.apps[0];
        assert_eq!(shell.label, "Shell");
        assert_eq!(shell.icon, "Bolt");
        assert_eq!(shell.color.as_deref(), Some("teal"));
        assert_eq!(shell.shortcut.as_deref(), Some("Mod+E"));
        assert_eq!(
            shell.command.as_deref(),
            Some("cd /work/northwind && ls /Users/demo/x")
        );
        let browser = &config.apps[1];
        assert_eq!(browser.url.as_deref(), Some("https://example.com/admin"));
        assert_eq!(browser.args.as_deref(), Some("/work/northwind"));
        assert_eq!(browser.label, "Open in Browser");
    }

    #[test]
    fn empty_overrides_fall_back_to_the_defaults() {
        let config = resolved_configured(json!({
            "apps": [{ "preset": "editor", "label": "", "icon": "", "app": "", "shortcut": "" }],
        }));
        let editor = &config.apps[0];
        assert_eq!(editor.label, "Edit Code");
        assert_eq!(editor.icon, "Code");
        assert_eq!(editor.app.as_deref(), Some("Zed"));
        assert_eq!(editor.shortcut.as_deref(), Some("Mod+O"));
        assert_eq!(editor.color, None);
    }

    #[test]
    fn a_builtins_edit_data_holds_the_overrides_as_written_and_the_defaults() {
        let config = resolved(json!({
            "apps": [
                42,
                { "preset": "terminal", "label": "Shell", "command": "cd ${dir}", "hiddenStates": ["x"] },
            ],
        }));
        let edit = &config.apps[0].edit;
        assert_eq!(
            edit.source,
            ActionSource {
                list: ActionList::Apps,
                index: 1
            }
        );
        assert_eq!(edit.preset, Some(Builtin::Terminal));
        assert_eq!(
            edit.draft,
            ActionDraft {
                runs_as: RunsAs::Terminal,
                label: "Shell".into(),
                command: Some("cd ${dir}".into()),
                ..ActionDraft::default()
            }
        );
        assert_eq!(
            edit.defaults,
            Some(ActionDraft {
                runs_as: RunsAs::Terminal,
                label: "Open Terminal".into(),
                command: Some(String::new()),
                icon: Some("Terminal".into()),
                shortcut: Some("Mod+T".into()),
                ..ActionDraft::default()
            })
        );
        assert_eq!(config.apps[0].hidden_states, ["x"]);
    }

    #[test]
    fn a_bare_builtin_has_an_empty_draft_and_defaults_for_its_target() {
        let config = resolved_configured(json!({ "apps": ["editor"] }));
        let edit = &config.apps[0].edit;
        assert_eq!(
            edit.draft,
            ActionDraft {
                runs_as: RunsAs::App,
                ..ActionDraft::default()
            }
        );
        assert_eq!(
            edit.defaults,
            Some(ActionDraft {
                runs_as: RunsAs::App,
                label: "Edit Code".into(),
                app: Some("Zed".into()),
                icon: Some("Code".into()),
                shortcut: Some("Mod+O".into()),
                ..ActionDraft::default()
            })
        );
    }

    #[test]
    fn a_source_index_counts_skipped_entries() {
        let config = resolved(json!({
            "apps": [
                "editor",
                "terminal",
                { "app": "Figma" },
                { "label": "Figma", "app": "Figma" },
            ],
        }));
        let indexes: Vec<usize> = config.apps.iter().map(|a| a.edit.source.index).collect();
        assert_eq!(indexes, [1, 3]);
    }

    #[test]
    fn builtins_list_all_six_in_order_and_flag_the_enabled_ones() {
        let config =
            resolved_configured(json!({ "apps": ["editor", { "preset": "git" }, "terminal"] }));
        let rows: Vec<_> = config
            .builtins
            .iter()
            .map(|b| (b.preset, b.label.as_str(), b.icon.as_str(), b.enabled))
            .collect();
        assert_eq!(
            rows,
            vec![
                (Builtin::Editor, "Edit Code", "Code", true),
                (Builtin::Terminal, "Open Terminal", "Terminal", true),
                (Builtin::Git, "Open Git Client", "CheckList", true),
                (Builtin::Browser, "Open in Browser", "Globe", false),
                (Builtin::RepoBrowser, "Open Repository", "Link", false),
                (Builtin::Claude, "Claude Code", "Stars", false),
            ]
        );
    }

    #[test]
    fn custom_apps_resolve_how_they_run() {
        let config = resolved(json!({
            "meta": { "url": "https://example.com" },
            "apps": [
                { "label": "Figma", "app": "Figma", "args": "${dir}" },
                { "label": "Docs", "url": "${url}/docs", "icon": "Book", "color": "blue", "shortcut": "cmd+d" },
                { "label": "Shell" },
                { "app": "Figma" },
                7,
                "nope",
                { "label": "Seeded", "command": "", "hiddenStates": ["a", 3, "b"] },
                { "label": "Build", "command": "make -C ${dir}" },
                { "label": 5, "app": "Figma" },
            ],
        }));
        assert_eq!(config.apps.len(), 5);

        let figma = &config.apps[0];
        assert_eq!(figma.label, "Figma");
        assert_eq!(figma.icon, "Window");
        assert_eq!(figma.runs_as, RunsAs::App);
        assert_eq!(figma.app.as_deref(), Some("Figma"));
        assert_eq!(figma.args.as_deref(), Some("/work/northwind"));
        assert_eq!(figma.command, None);
        assert_eq!(figma.shortcut, None);
        assert_eq!(figma.edit.preset, None);
        assert_eq!(figma.edit.defaults, None);
        assert_eq!(
            figma.edit.source,
            ActionSource {
                list: ActionList::Apps,
                index: 0
            }
        );
        assert_eq!(
            figma.edit.draft,
            ActionDraft {
                runs_as: RunsAs::App,
                label: "Figma".into(),
                app: Some("Figma".into()),
                args: Some("${dir}".into()),
                ..ActionDraft::default()
            }
        );

        let docs = &config.apps[1];
        assert_eq!(docs.runs_as, RunsAs::Url);
        assert_eq!(docs.url.as_deref(), Some("https://example.com/docs"));
        assert_eq!(docs.icon, "Book");
        assert_eq!(docs.color.as_deref(), Some("blue"));
        assert_eq!(docs.shortcut.as_deref(), Some("Mod+D"));

        let shell = &config.apps[2];
        assert_eq!(shell.runs_as, RunsAs::Terminal);
        assert_eq!(shell.command, None);

        let seeded = &config.apps[3];
        assert_eq!(seeded.runs_as, RunsAs::Terminal);
        assert_eq!(seeded.command.as_deref(), Some(""));
        assert_eq!(seeded.hidden_states, ["a", "b"]);
        assert_eq!(seeded.edit.source.index, 6);

        let build = &config.apps[4];
        assert_eq!(build.command.as_deref(), Some("make -C /work/northwind"));
        assert_eq!(build.edit.source.index, 7);
    }

    #[test]
    fn a_custom_app_with_an_empty_app_runs_by_its_url() {
        let config = resolved(
            json!({ "apps": [{ "label": "Docs", "app": "", "url": "https://example.com" }] }),
        );
        assert_eq!(config.apps[0].runs_as, RunsAs::Url);
    }

    #[test]
    fn scripts_need_a_label_and_a_command() {
        let config = resolved(json!({
            "scripts": [
                { "label": "Build", "command": "make -C ${dir}" },
                { "label": "No command" },
                { "command": "ls" },
                { "label": "Odd", "command": 3 },
                "build",
                {
                    "label": "Deploy",
                    "command": "~/bin/deploy",
                    "icon": "Rocket",
                    "shortcut": "cmd+d",
                    "hiddenStates": ["live"],
                },
            ],
        }));
        assert_eq!(config.scripts.len(), 2);

        let build = &config.scripts[0];
        assert_eq!(build.label, "Build");
        assert_eq!(build.icon, "Terminal");
        assert_eq!(build.runs_as, RunsAs::Script);
        assert_eq!(build.command.as_deref(), Some("make -C /work/northwind"));
        assert_eq!(
            (
                build.app.as_deref(),
                build.args.as_deref(),
                build.url.as_deref()
            ),
            (None, None, None)
        );
        assert_eq!(
            build.edit.source,
            ActionSource {
                list: ActionList::Scripts,
                index: 0
            }
        );
        assert_eq!(build.edit.preset, None);

        let deploy = &config.scripts[1];
        assert_eq!(deploy.icon, "Rocket");
        assert_eq!(deploy.shortcut.as_deref(), Some("Mod+D"));
        assert_eq!(deploy.command.as_deref(), Some("/Users/demo/bin/deploy"));
        assert_eq!(deploy.hidden_states, ["live"]);
        assert_eq!(
            deploy.edit.source,
            ActionSource {
                list: ActionList::Scripts,
                index: 5
            }
        );
        assert_eq!(deploy.edit.draft.runs_as, RunsAs::Script);
        assert_eq!(deploy.edit.draft.command.as_deref(), Some("~/bin/deploy"));
    }

    #[test]
    fn env_keeps_file_order_turns_scalars_into_text_and_fills_values() {
        let config = resolved(json!({
            "env": {
                "ZED": "last-alphabetically-first",
                "PORT": 8080,
                "DEBUG": true,
                "RATIO": 1.5,
                "LIST": [1, 2],
                "NESTED": { "a": 1 },
                "EMPTY": null,
                "CLAUDE_CONFIG_DIR": "~/.claude-demo",
                "ROOT": "${dir}/vendor",
            },
        }));
        let pairs: Vec<(&str, &str)> = config
            .env
            .iter()
            .map(|v| (v.name.as_str(), v.value.as_str()))
            .collect();
        assert_eq!(
            pairs,
            vec![
                ("ZED", "last-alphabetically-first"),
                ("PORT", "8080"),
                ("DEBUG", "true"),
                ("RATIO", "1.5"),
                ("CLAUDE_CONFIG_DIR", "/Users/demo/.claude-demo"),
                ("ROOT", "/work/northwind/vendor"),
            ]
        );
    }

    #[test]
    fn claude_setup_exists_only_for_folders_with_a_claude_directory() {
        let file = json!({ "env": { "CLAUDE_CONFIG_DIR": "~/.claude-demo" } });
        let with = resolve_in(file.clone(), &Settings::default(), false, true);
        assert_eq!(
            with.claude,
            Some(ClaudeSetup {
                config_dir: Some("/Users/demo/.claude-demo".into())
            })
        );
        let without = resolve_in(file, &Settings::default(), false, false);
        assert_eq!(without.claude, None);
        let default_profile = resolve_in(json!({}), &Settings::default(), false, true);
        assert_eq!(
            default_profile.claude,
            Some(ClaudeSetup { config_dir: None })
        );
    }

    #[test]
    fn state_providers_keep_order_and_fill_their_commands() {
        let config = resolved(json!({
            "stateProviders": {
                "zeta": "git -C ${dir} branch --show-current",
                "alpha": "cat ~/status",
                "broken": 5,
            },
        }));
        assert_eq!(
            config.state_providers,
            vec![
                StateProvider {
                    name: "zeta".into(),
                    command: "git -C /work/northwind branch --show-current".into()
                },
                StateProvider {
                    name: "alpha".into(),
                    command: "cat /Users/demo/status".into()
                },
            ]
        );
    }

    #[test]
    fn states_keep_order_fall_back_to_their_name_and_skip_incomplete_values() {
        let config = resolved(json!({
            "states": {
                "deploy": {
                    "source": "branch:.name",
                    "label": "Deploy",
                    "values": {
                        "live": { "value": "yes", "label": "Live", "color": "green" },
                        "noLabel": { "value": "no" },
                        "noValue": { "label": "Down" },
                        "down": { "value": "no", "label": "Down" },
                    },
                },
                "plain": { "source": "curl-free ${dir}" },
                "orphan": { "label": "Orphan" },
            },
        }));
        assert_eq!(
            config.states,
            vec![
                StateDef {
                    name: "deploy".into(),
                    source: "branch:.name".into(),
                    label: "Deploy".into(),
                    values: vec![
                        StateValue {
                            key: "live".into(),
                            value: "yes".into(),
                            label: "Live".into(),
                            color: Some("green".into()),
                        },
                        StateValue {
                            key: "down".into(),
                            value: "no".into(),
                            label: "Down".into(),
                            color: None
                        },
                    ],
                },
                StateDef {
                    name: "plain".into(),
                    source: "curl-free /work/northwind".into(),
                    label: "plain".into(),
                    values: vec![],
                },
            ]
        );
    }

    #[test]
    fn dev_database_locked_and_git_pass_through() {
        let config = resolve_in(
            json!({ "dev": { "ddev": true }, "database": false, "locked": true }),
            &Settings::default(),
            true,
            false,
        );
        assert_eq!(config.dev, DevMode::Ddev);
        assert_eq!(config.database, DatabaseSetting::Hidden);
        assert!(config.locked);
        assert!(config.is_git_repo);
    }

    #[test]
    fn locked_needs_a_true_boolean() {
        for locked in [json!("true"), json!(1), json!(false), json!(null)] {
            let config = resolved(json!({ "locked": locked }));
            assert!(!config.locked, "{locked}");
        }
    }

    #[test]
    fn a_file_with_only_a_name_resolves_to_empty_lists() {
        let config = resolved(json!({ "name": "Northwind" }));
        assert!(config.apps.is_empty());
        assert!(config.scripts.is_empty());
        assert!(config.env.is_empty());
        assert!(config.state_providers.is_empty());
        assert!(config.states.is_empty());
        assert_eq!(config.dev, DevMode::None);
        assert_eq!(config.database, DatabaseSetting::Detect);
        assert_eq!(config.claude, None);
        assert!(!config.locked);
        assert!(!config.is_git_repo);
        assert_eq!(config.builtins.len(), 6);
        assert!(config.builtins.iter().all(|b| !b.enabled));
    }

    #[test]
    fn lists_of_the_wrong_type_read_as_empty() {
        let config = resolved(
            json!({ "apps": "editor", "scripts": {}, "env": [1], "states": [], "stateProviders": 3 }),
        );
        assert!(config.apps.is_empty());
        assert!(config.scripts.is_empty());
        assert!(config.env.is_empty());
        assert!(config.states.is_empty());
        assert!(config.state_providers.is_empty());
    }

    #[test]
    fn the_resolved_config_serializes_camel_case_for_the_page() {
        let value = serde_json::to_value(resolved(json!({ "apps": ["terminal"] }))).unwrap();
        let object = value.as_object().unwrap();
        for key in [
            "apps",
            "scripts",
            "builtins",
            "env",
            "stateProviders",
            "states",
            "dev",
            "database",
            "claude",
            "locked",
            "isGitRepo",
        ] {
            assert!(object.contains_key(key), "{key}");
        }
        let action = &value["apps"][0];
        assert_eq!(action["runsAs"], json!("terminal"));
        assert_eq!(action["hiddenStates"], json!([]));
        assert_eq!(
            action["edit"]["source"],
            json!({ "list": "apps", "index": 0 })
        );
        assert_eq!(action["edit"]["preset"], json!("terminal"));
        assert_eq!(
            value["builtins"][0]["unavailableReason"],
            json!("Choose a default editor in Settings")
        );
    }
}
