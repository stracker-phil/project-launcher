//! The launch config's vocabulary, shared by the resolver, the editor and the page: built-in apps, how an
//! action runs, where it lives in the file and the draft the action editor works on.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use ts_rs::TS;

use crate::menu::keys::Combo;

/// An app every project can show, written in `apps` as its name (`"editor"`) or as
/// `{ "preset": "editor", … }` with overrides.
#[derive(Serialize, Deserialize, TS, Debug, Clone, Copy, PartialEq, Eq)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub enum Builtin {
    Editor,
    Terminal,
    Git,
    Browser,
    RepoBrowser,
    Claude,
}

impl Builtin {
    /// The order the action editor lists them in.
    pub const ALL: [Builtin; 6] = [
        Builtin::Editor,
        Builtin::Terminal,
        Builtin::Git,
        Builtin::Browser,
        Builtin::RepoBrowser,
        Builtin::Claude,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Builtin::Editor => "editor",
            Builtin::Terminal => "terminal",
            Builtin::Git => "git",
            Builtin::Browser => "browser",
            Builtin::RepoBrowser => "repoBrowser",
            Builtin::Claude => "claude",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Builtin::Editor => "Edit Code",
            Builtin::Terminal => "Open Terminal",
            Builtin::Git => "Open Git Client",
            Builtin::Browser => "Open in Browser",
            Builtin::RepoBrowser => "Open Repository",
            Builtin::Claude => "Claude Code",
        }
    }

    pub fn icon(self) -> &'static str {
        match self {
            Builtin::Editor => "Code",
            Builtin::Terminal => "Terminal",
            Builtin::Git => "CheckList",
            Builtin::Browser => "Globe",
            Builtin::RepoBrowser => "Link",
            Builtin::Claude => "Stars",
        }
    }

    /// The default key, in the file's notation.
    pub fn shortcut(self) -> &'static str {
        match self {
            Builtin::Editor => "cmd+o",
            Builtin::Terminal => "cmd+t",
            Builtin::Git => "cmd+g",
            Builtin::Browser => "cmd+b",
            Builtin::RepoBrowser => "cmd+shift+r",
            Builtin::Claude => "cmd+shift+k",
        }
    }

    fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|b| b.name() == name)
    }

    /// The built-in an `apps` entry refers to: `"editor"` or `{ "preset": "editor", … }`.
    pub fn of(entry: &Value) -> Option<Self> {
        match entry {
            Value::String(name) => Self::from_name(name),
            Value::Object(object) => object.get("preset")?.as_str().and_then(Self::from_name),
            _ => None,
        }
    }
}

/// How an action runs; a `script` lives in `scripts`, everything else in `apps`.
#[derive(Serialize, Deserialize, TS, Debug, Clone, Copy, PartialEq, Eq, Default)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub enum RunsAs {
    #[default]
    App,
    Url,
    Terminal,
    Script,
}

#[derive(Serialize, Deserialize, TS, Debug, Clone, Copy, PartialEq, Eq)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub enum ActionList {
    Apps,
    Scripts,
}

impl ActionList {
    pub fn key(self) -> &'static str {
        match self {
            ActionList::Apps => "apps",
            ActionList::Scripts => "scripts",
        }
    }
}

/// Where an action lives in the file.
#[derive(Serialize, Deserialize, TS, Debug, Clone, Copy, PartialEq, Eq)]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct ActionSource {
    pub list: ActionList,
    pub index: usize,
}

/// The editable fields of an action as written in the file, before `${dir}` and the like are filled in. The
/// shortcut is in the page's notation (`Mod+Shift+K`).
#[derive(Serialize, Deserialize, TS, Debug, Clone, PartialEq, Default)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct ActionDraft {
    pub runs_as: RunsAs,
    pub label: String,
    #[ts(optional)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub app: Option<String>,
    #[ts(optional)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub args: Option<String>,
    #[ts(optional)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[ts(optional)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[ts(optional)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[ts(optional)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[ts(optional)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shortcut: Option<String>,
}

/// The fields a draft carries besides `label`, in the file's spelling.
pub const DRAFT_FIELDS: [&str; 7] = ["app", "args", "url", "command", "icon", "color", "shortcut"];

impl ActionDraft {
    pub fn field(&self, name: &str) -> Option<&str> {
        match name {
            "label" => Some(&self.label),
            "app" => self.app.as_deref(),
            "args" => self.args.as_deref(),
            "url" => self.url.as_deref(),
            "command" => self.command.as_deref(),
            "icon" => self.icon.as_deref(),
            "color" => self.color.as_deref(),
            "shortcut" => self.shortcut.as_deref(),
            _ => None,
        }
    }

    /// This draft's fields with `defaults` filling the empty ones: what a built-in's copy holds.
    pub fn over(self, defaults: &ActionDraft) -> ActionDraft {
        let pick = |own: Option<String>, default: &Option<String>| {
            own.filter(|v| !v.is_empty()).or_else(|| default.clone())
        };
        ActionDraft {
            runs_as: self.runs_as,
            label: if self.label.is_empty() {
                defaults.label.clone()
            } else {
                self.label
            },
            app: pick(self.app, &defaults.app),
            args: pick(self.args, &defaults.args),
            url: pick(self.url, &defaults.url),
            command: pick(self.command, &defaults.command),
            icon: pick(self.icon, &defaults.icon),
            color: pick(self.color, &defaults.color),
            shortcut: pick(self.shortcut, &defaults.shortcut),
        }
    }

    /// The draft of a file entry: its string fields as written, the shortcut in the page's notation.
    pub fn of_entry(entry: Option<&Value>, runs_as: RunsAs) -> Self {
        let text = |name: &str| entry?.get(name)?.as_str().map(str::to_string);
        ActionDraft {
            runs_as,
            label: text("label").unwrap_or_default(),
            app: text("app"),
            args: text("args"),
            url: text("url"),
            command: text("command"),
            icon: text("icon"),
            color: text("color"),
            shortcut: text("shortcut").as_deref().and_then(page_key),
        }
    }
}

/// A key from the file (`cmd+shift+r`) in the page's notation (`Mod+Shift+R`); `None` when it isn't one.
pub fn page_key(file_key: &str) -> Option<String> {
    Combo::parse(file_key).map(|combo| combo.binding())
}

/// A key in either notation as the file writes it (`cmd+shift+k`).
pub fn file_key(key: &str) -> Option<String> {
    let combo = Combo::parse(key)?;
    let mut parts: Vec<String> = [
        (combo.command, "cmd"),
        (combo.control, "ctrl"),
        (combo.option, "alt"),
        (combo.shift, "shift"),
    ]
    .into_iter()
    .filter(|(on, _)| *on)
    .map(|(_, name)| name.to_string())
    .collect();
    parts.push(combo.key.to_ascii_lowercase());
    Some(parts.join("+"))
}

/// What a project needs running while the owner works on it; `ddev` and `wp-env` run inside Docker.
#[derive(Serialize, Deserialize, TS, Debug, Clone, Copy, PartialEq, Eq, Default)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "kebab-case")]
pub enum DevMode {
    #[default]
    None,
    Docker,
    Ddev,
    WpEnv,
}

impl DevMode {
    /// The file's `dev`: `{ "ddev": true }` wins over `{ "wpEnv": true }`, which wins over `{ "docker": true }`.
    pub fn of(dev: Option<&Value>) -> Self {
        let on = |key: &str| dev.and_then(|d| d.get(key)) == Some(&Value::Bool(true));
        if on("ddev") {
            DevMode::Ddev
        } else if on("wpEnv") {
            DevMode::WpEnv
        } else if on("docker") {
            DevMode::Docker
        } else {
            DevMode::None
        }
    }

    /// The `dev` value that means this mode; `None` removes the key.
    pub fn value(self) -> Option<Value> {
        let key = match self {
            DevMode::None => return None,
            DevMode::Docker => "docker",
            DevMode::Ddev => "ddev",
            DevMode::WpEnv => "wpEnv",
        };
        Some(serde_json::json!({ key: true }))
    }
}

/// The file's `database`: absent finds the database in the dev server's setup, `false` hides it, a
/// `mysql://` URL names it. The URL holds a password, so it never reaches the page.
#[derive(Serialize, TS, Debug, Clone, Copy, PartialEq, Eq, Default)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub enum DatabaseSetting {
    #[default]
    Detect,
    Hidden,
    Url,
}

impl DatabaseSetting {
    pub fn of(database: Option<&Value>) -> Self {
        match database {
            Some(Value::Bool(false)) => DatabaseSetting::Hidden,
            Some(Value::String(url)) if !url.trim().is_empty() => DatabaseSetting::Url,
            _ => DatabaseSetting::Detect,
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn a_builtin_is_found_by_name_or_preset_object() {
        for (entry, expected) in [
            (json!("editor"), Some(Builtin::Editor)),
            (json!("repoBrowser"), Some(Builtin::RepoBrowser)),
            (
                json!({ "preset": "repoBrowser" }),
                Some(Builtin::RepoBrowser),
            ),
            (
                json!({ "preset": "git", "label": "Fork" }),
                Some(Builtin::Git),
            ),
            (json!("unknown"), None),
            (json!({ "preset": "unknown" }), None),
            (json!({ "preset": 3 }), None),
            (json!({ "label": "Editor" }), None),
            (json!(3), None),
            (json!(null), None),
            (json!(["editor"]), None),
        ] {
            assert_eq!(Builtin::of(&entry), expected, "{entry}");
        }
    }

    #[test]
    fn builtins_are_listed_in_the_editors_order() {
        let names: Vec<&str> = Builtin::ALL.into_iter().map(Builtin::name).collect();
        assert_eq!(
            names,
            [
                "editor",
                "terminal",
                "git",
                "browser",
                "repoBrowser",
                "claude"
            ]
        );
    }

    #[test]
    fn dev_mode_prefers_ddev_then_wp_env_then_docker() {
        for (dev, expected) in [
            (
                json!({ "ddev": true, "wpEnv": true, "docker": true }),
                DevMode::Ddev,
            ),
            (json!({ "wpEnv": true, "docker": true }), DevMode::WpEnv),
            (json!({ "docker": true }), DevMode::Docker),
            (json!({ "ddev": false, "docker": true }), DevMode::Docker),
            (json!({ "ddev": "true" }), DevMode::None),
            (json!({ "ddev": 1 }), DevMode::None),
            (json!({}), DevMode::None),
            (json!(true), DevMode::None),
        ] {
            assert_eq!(DevMode::of(Some(&dev)), expected, "{dev}");
        }
        assert_eq!(DevMode::of(None), DevMode::None);
    }

    #[test]
    fn dev_mode_value_round_trips_and_none_removes_the_key() {
        for mode in [DevMode::Docker, DevMode::Ddev, DevMode::WpEnv] {
            let value = mode.value();
            assert_eq!(DevMode::of(value.as_ref()), mode, "{mode:?}");
        }
        assert_eq!(DevMode::Ddev.value(), Some(json!({ "ddev": true })));
        assert_eq!(DevMode::WpEnv.value(), Some(json!({ "wpEnv": true })));
        assert_eq!(DevMode::None.value(), None);
    }

    #[test]
    fn dev_mode_serializes_as_the_pages_names() {
        for (mode, name) in [
            (DevMode::None, "none"),
            (DevMode::Docker, "docker"),
            (DevMode::Ddev, "ddev"),
            (DevMode::WpEnv, "wp-env"),
        ] {
            assert_eq!(serde_json::to_value(mode).unwrap(), json!(name), "{mode:?}");
        }
    }

    #[test]
    fn database_setting_reads_absent_false_and_urls() {
        for (database, expected) in [
            (None, DatabaseSetting::Detect),
            (Some(json!(false)), DatabaseSetting::Hidden),
            (Some(json!(true)), DatabaseSetting::Detect),
            (
                Some(json!("mysql://demo:secret@127.0.0.1:3306/northwind")),
                DatabaseSetting::Url,
            ),
            (Some(json!("")), DatabaseSetting::Detect),
            (Some(json!("   ")), DatabaseSetting::Detect),
            (Some(json!(3)), DatabaseSetting::Detect),
            (Some(json!(null)), DatabaseSetting::Detect),
        ] {
            assert_eq!(
                DatabaseSetting::of(database.as_ref()),
                expected,
                "{database:?}"
            );
        }
    }

    #[test]
    fn database_setting_serializes_without_the_url() {
        assert_eq!(
            serde_json::to_value(DatabaseSetting::Url).unwrap(),
            json!("url")
        );
        assert_eq!(
            serde_json::to_value(DatabaseSetting::Hidden).unwrap(),
            json!("hidden")
        );
        assert_eq!(
            serde_json::to_value(DatabaseSetting::Detect).unwrap(),
            json!("detect")
        );
    }

    #[test]
    fn a_file_key_becomes_the_pages_key() {
        for (input, expected) in [
            ("cmd+shift+r", Some("Mod+Shift+R")),
            ("cmd+o", Some("Mod+O")),
            ("ctrl+alt+shift+k", Some("Ctrl+Alt+Shift+K")),
            ("Mod+Shift+R", Some("Mod+Shift+R")),
            ("cmd+shift", None),
            ("cmd+a+b", None),
            ("nope", None),
            ("", None),
        ] {
            assert_eq!(page_key(input).as_deref(), expected, "{input}");
        }
    }

    #[test]
    fn a_key_in_either_notation_becomes_the_files_key() {
        for (input, expected) in [
            ("Mod+Shift+L", Some("cmd+shift+l")),
            ("cmd+shift+l", Some("cmd+shift+l")),
            ("Mod+Ctrl+Alt+Shift+K", Some("cmd+ctrl+alt+shift+k")),
            ("shift+cmd+k", Some("cmd+shift+k")),
            ("Ctrl+J", Some("ctrl+j")),
            ("Mod+", None),
            ("Mod+Shift", None),
            ("nope", None),
            ("", None),
        ] {
            assert_eq!(file_key(input).as_deref(), expected, "{input}");
        }
    }

    #[test]
    fn a_draft_reads_string_fields_as_written_and_converts_the_shortcut() {
        let entry = json!({
            "label": "Figma",
            "app": "Figma",
            "args": "${dir}",
            "url": "${url}/x",
            "command": "ls ~",
            "icon": "Palette",
            "color": "blue",
            "shortcut": "cmd+shift+f",
            "hiddenStates": ["a"],
        });
        assert_eq!(
            ActionDraft::of_entry(Some(&entry), RunsAs::App),
            ActionDraft {
                runs_as: RunsAs::App,
                label: "Figma".into(),
                app: Some("Figma".into()),
                args: Some("${dir}".into()),
                url: Some("${url}/x".into()),
                command: Some("ls ~".into()),
                icon: Some("Palette".into()),
                color: Some("blue".into()),
                shortcut: Some("Mod+Shift+F".into()),
            }
        );
    }

    #[test]
    fn a_draft_of_no_entry_or_odd_fields_is_empty() {
        assert_eq!(
            ActionDraft::of_entry(None, RunsAs::Url),
            ActionDraft {
                runs_as: RunsAs::Url,
                ..ActionDraft::default()
            }
        );
        let entry = json!({ "label": 3, "app": null, "shortcut": "nope" });
        assert_eq!(
            ActionDraft::of_entry(Some(&entry), RunsAs::Terminal),
            ActionDraft {
                runs_as: RunsAs::Terminal,
                ..ActionDraft::default()
            }
        );
    }

    #[test]
    fn a_draft_over_defaults_keeps_its_own_values_and_fills_the_empty_ones() {
        let defaults = ActionDraft {
            runs_as: RunsAs::App,
            label: "Edit Code".into(),
            app: Some("Zed".into()),
            icon: Some("Code".into()),
            shortcut: Some("Mod+O".into()),
            ..ActionDraft::default()
        };
        let own = ActionDraft {
            runs_as: RunsAs::Terminal,
            label: String::new(),
            app: Some("Fork".into()),
            icon: Some(String::new()),
            color: Some("Red".into()),
            ..ActionDraft::default()
        };

        assert_eq!(
            own.over(&defaults),
            ActionDraft {
                runs_as: RunsAs::Terminal,
                label: "Edit Code".into(),
                app: Some("Fork".into()),
                icon: Some("Code".into()),
                color: Some("Red".into()),
                shortcut: Some("Mod+O".into()),
                ..ActionDraft::default()
            }
        );
        let named = ActionDraft {
            label: "Mine".into(),
            ..ActionDraft::default()
        };
        assert_eq!(named.over(&defaults).label, "Mine");
    }

    #[test]
    fn a_draft_serializes_camel_case_and_omits_missing_fields() {
        let draft = ActionDraft {
            runs_as: RunsAs::Terminal,
            label: "Shell".into(),
            command: Some("ls".into()),
            ..ActionDraft::default()
        };
        assert_eq!(
            serde_json::to_value(&draft).unwrap(),
            json!({ "runsAs": "terminal", "label": "Shell", "command": "ls" })
        );
    }

    #[test]
    fn a_draft_from_the_page_may_omit_optional_fields() {
        let draft: ActionDraft = serde_json::from_value(
            json!({ "runsAs": "url", "label": "Docs", "url": "https://example.com" }),
        )
        .unwrap();
        assert_eq!(
            draft,
            ActionDraft {
                runs_as: RunsAs::Url,
                label: "Docs".into(),
                url: Some("https://example.com".into()),
                ..ActionDraft::default()
            }
        );
    }

    #[test]
    fn an_action_source_serializes_its_list_in_camel_case() {
        let source = ActionSource {
            list: ActionList::Scripts,
            index: 2,
        };
        assert_eq!(
            serde_json::to_value(source).unwrap(),
            json!({ "list": "scripts", "index": 2 })
        );
    }
}
