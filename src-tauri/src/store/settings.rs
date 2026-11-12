//! `settings.json`: the app's settings and the tag list, repaired field by field on load (ADR 0013).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use ts_rs::TS;

/// Automatic follows macOS; Light and Dark are fixed (ADR 0026).
#[derive(Serialize, Deserialize, TS, Debug, Clone, Copy, PartialEq, Eq, Default)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "lowercase")]
pub enum Appearance {
    #[default]
    System,
    Light,
    Dark,
}

/// The color of selection, focus, switches and primary buttons: the macOS accent color or one of nine named
/// hues (ADR 0042). Project colors are separate.
#[derive(Serialize, Deserialize, TS, Debug, Clone, Copy, PartialEq, Eq, Default)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "lowercase")]
pub enum Accent {
    #[default]
    System,
    Orange,
    Gold,
    Red,
    Magenta,
    Purple,
    Indigo,
    Blue,
    Teal,
    Graphite,
}

/// The owner's key per command id; `None` leaves the command without one.
pub type Shortcuts = BTreeMap<String, Option<String>>;

#[derive(Serialize, Deserialize, TS, Debug, Clone, PartialEq, Eq)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub appearance: Appearance,
    pub accent: Accent,
    /// Every tag in the owner's order, including tags no project carries.
    pub tags: Vec<String>,
    /// The owner's key for a command id, overriding its default; `null` leaves the command without one.
    pub shortcuts: Shortcuts,
    /// The editor for projects that name none; empty uses the system's default app.
    pub default_editor: String,
    pub git_client: String,
    /// The app that opens terminal sessions (`.command` files).
    pub terminal_app: String,
    /// The app that opens `.project-launcher.json`; empty uses the system's default app.
    pub config_editor: String,
    /// The app that starts and stops Docker.
    pub docker_manager: DockerManager,
    /// The custom manager's commands.
    pub docker_start_command: String,
    pub docker_stop_command: String,
}

#[derive(Serialize, Deserialize, TS, Debug, Clone, Copy, PartialEq, Eq, Default)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "kebab-case")]
pub enum DockerManager {
    #[default]
    DockerDesktop,
    #[serde(rename = "orbstack")]
    OrbStack,
    Colima,
    Custom,
}

const DEFAULT_TERMINAL: &str = "Terminal";

/// The apps Settings → General names: what the built-ins and the project file open with.
#[derive(Serialize, Deserialize, TS, Debug, Clone, Copy, PartialEq, Eq)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub enum AppSetting {
    DefaultEditor,
    GitClient,
    TerminalApp,
    ConfigEditor,
    DockerStartCommand,
    DockerStopCommand,
}

impl AppSetting {
    pub fn get(self, settings: &Settings) -> &str {
        match self {
            AppSetting::DefaultEditor => &settings.default_editor,
            AppSetting::GitClient => &settings.git_client,
            AppSetting::TerminalApp => &settings.terminal_app,
            AppSetting::ConfigEditor => &settings.config_editor,
            AppSetting::DockerStartCommand => &settings.docker_start_command,
            AppSetting::DockerStopCommand => &settings.docker_stop_command,
        }
    }

    pub fn of(self, settings: &mut Settings) -> &mut String {
        match self {
            AppSetting::DefaultEditor => &mut settings.default_editor,
            AppSetting::GitClient => &mut settings.git_client,
            AppSetting::TerminalApp => &mut settings.terminal_app,
            AppSetting::ConfigEditor => &mut settings.config_editor,
            AppSetting::DockerStartCommand => &mut settings.docker_start_command,
            AppSetting::DockerStopCommand => &mut settings.docker_stop_command,
        }
    }

    /// The setting's key in `settings.json`, the last part of its field path.
    pub fn key(self) -> &'static str {
        match self {
            AppSetting::DefaultEditor => "defaultEditor",
            AppSetting::GitClient => "gitClient",
            AppSetting::TerminalApp => "terminalApp",
            AppSetting::ConfigEditor => "configEditor",
            AppSetting::DockerStartCommand => "dockerStartCommand",
            AppSetting::DockerStopCommand => "dockerStopCommand",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            AppSetting::DefaultEditor => "Change Default Editor",
            AppSetting::GitClient => "Change Git Client",
            AppSetting::TerminalApp => "Change Terminal App",
            AppSetting::ConfigEditor => "Change Config Editor",
            AppSetting::DockerStartCommand => "Change Docker Start Command",
            AppSetting::DockerStopCommand => "Change Docker Stop Command",
        }
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            appearance: Appearance::System,
            accent: Accent::System,
            tags: Vec::new(),
            shortcuts: BTreeMap::new(),
            default_editor: String::new(),
            git_client: String::new(),
            terminal_app: DEFAULT_TERMINAL.into(),
            config_editor: String::new(),
            docker_manager: DockerManager::default(),
            docker_start_command: String::new(),
            docker_stop_command: String::new(),
        }
    }
}

impl Settings {
    /// Settings from whatever `settings.json` holds: each field that doesn't fit falls back to its default,
    /// unknown keys are dropped.
    pub fn repaired(raw: Value) -> Self {
        let defaults = Self::default();
        let Value::Object(v) = raw else {
            return defaults;
        };
        let text = |key: &str, default: String| match v.get(key) {
            Some(Value::String(s)) => s.trim().to_string(),
            _ => default,
        };
        let terminal_app = Some(text("terminalApp", String::new()))
            .filter(|s| !s.is_empty())
            .unwrap_or(defaults.terminal_app);
        Self {
            appearance: v
                .get("appearance")
                .and_then(|a| Appearance::deserialize(a).ok())
                .unwrap_or_default(),
            accent: v
                .get("accent")
                .and_then(|a| Accent::deserialize(a).ok())
                .unwrap_or_default(),
            tags: v.get("tags").map(repaired_tags).unwrap_or_default(),
            shortcuts: v
                .get("shortcuts")
                .map(repaired_shortcuts)
                .unwrap_or_default(),
            default_editor: text("defaultEditor", defaults.default_editor),
            git_client: text("gitClient", defaults.git_client),
            terminal_app,
            config_editor: text("configEditor", defaults.config_editor),
            docker_manager: v
                .get("dockerManager")
                .and_then(|m| DockerManager::deserialize(m).ok())
                .unwrap_or_default(),
            docker_start_command: text("dockerStartCommand", defaults.docker_start_command),
            docker_stop_command: text("dockerStopCommand", defaults.docker_stop_command),
        }
    }
}

/// Trimmed, non-empty strings, each once, in their first position.
fn repaired_tags(raw: &Value) -> Vec<String> {
    let mut tags: Vec<String> = Vec::new();
    for tag in raw.as_array().into_iter().flatten() {
        if let Some(tag) = tag.as_str().map(str::trim).filter(|t| !t.is_empty())
            && !tags.iter().any(|t| t == tag)
        {
            tags.push(tag.to_string());
        }
    }
    tags
}

/// Entries with a command id and a non-empty key or `null`.
fn repaired_shortcuts(raw: &Value) -> Shortcuts {
    let empty = Map::new();
    raw.as_object()
        .unwrap_or(&empty)
        .iter()
        .filter(|(command, _)| !command.trim().is_empty())
        .filter_map(|(command, key)| {
            let key = match key {
                Value::Null => None,
                Value::String(s) if !s.trim().is_empty() => Some(s.trim().to_string()),
                _ => return None,
            };
            Some((command.trim().to_string(), key))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn an_app_setting_reads_its_own_field() {
        let settings = Settings {
            default_editor: "Nova".into(),
            git_client: "Fork".into(),
            terminal_app: "Ghostty".into(),
            config_editor: "Zed".into(),
            docker_start_command: "demo-docker up".into(),
            docker_stop_command: "demo-docker down".into(),
            ..Settings::default()
        };
        for (setting, expected) in [
            (AppSetting::DefaultEditor, "Nova"),
            (AppSetting::GitClient, "Fork"),
            (AppSetting::TerminalApp, "Ghostty"),
            (AppSetting::ConfigEditor, "Zed"),
            (AppSetting::DockerStartCommand, "demo-docker up"),
            (AppSetting::DockerStopCommand, "demo-docker down"),
        ] {
            assert_eq!(setting.get(&settings), expected, "{setting:?}");
        }
    }

    #[test]
    fn a_non_object_loads_the_defaults() {
        for raw in [json!(null), json!([]), json!("dark"), json!(7)] {
            assert_eq!(
                Settings::repaired(raw.clone()),
                Settings::default(),
                "{raw}"
            );
        }
    }

    #[test]
    fn an_empty_object_loads_the_defaults() {
        assert_eq!(Settings::repaired(json!({})), Settings::default());
    }

    #[test]
    fn a_full_valid_object_is_kept() {
        let raw = json!({
            "appearance": "dark",
            "accent": "teal",
            "tags": ["client", "internal"],
            "shortcuts": {"app:logs": "Cmd+L", "app:data": null},
            "defaultEditor": "Zed",
            "gitClient": "Fork",
            "terminalApp": "Ghostty",
            "configEditor": "TextEdit",
            "dockerManager": "custom",
            "dockerStartCommand": "demo-docker up",
            "dockerStopCommand": "demo-docker down",
        });

        assert_eq!(
            Settings::repaired(raw),
            Settings {
                appearance: Appearance::Dark,
                accent: Accent::Teal,
                tags: vec!["client".into(), "internal".into()],
                shortcuts: BTreeMap::from([
                    ("app:logs".to_string(), Some("Cmd+L".to_string())),
                    ("app:data".to_string(), None),
                ]),
                default_editor: "Zed".into(),
                git_client: "Fork".into(),
                terminal_app: "Ghostty".into(),
                config_editor: "TextEdit".into(),
                docker_manager: DockerManager::Custom,
                docker_start_command: "demo-docker up".into(),
                docker_stop_command: "demo-docker down".into(),
            }
        );
    }

    #[test]
    fn the_docker_manager_accepts_its_four_values_and_falls_back_to_the_default() {
        for (input, expected) in [
            (json!("docker-desktop"), DockerManager::DockerDesktop),
            (json!("orbstack"), DockerManager::OrbStack),
            (json!("colima"), DockerManager::Colima),
            (json!("custom"), DockerManager::Custom),
            (json!("podman"), DockerManager::DockerDesktop),
            (json!("OrbStack"), DockerManager::DockerDesktop),
            (json!(3), DockerManager::DockerDesktop),
            (json!(null), DockerManager::DockerDesktop),
        ] {
            let repaired = Settings::repaired(json!({ "dockerManager": input.clone() }));
            assert_eq!(repaired.docker_manager, expected, "{input}");
        }
    }

    #[test]
    fn the_docker_commands_are_trimmed_and_may_be_empty() {
        let repaired = Settings::repaired(json!({
            "dockerStartCommand": "  demo-docker up  ",
            "dockerStopCommand": "   ",
        }));

        assert_eq!(repaired.docker_start_command, "demo-docker up");
        assert_eq!(repaired.docker_stop_command, "");
    }

    #[test]
    fn appearance_accepts_its_three_values_and_falls_back_to_system() {
        for (input, expected) in [
            (json!("system"), Appearance::System),
            (json!("light"), Appearance::Light),
            (json!("dark"), Appearance::Dark),
            (json!("sepia"), Appearance::System),
            (json!("Dark"), Appearance::System),
            (json!(3), Appearance::System),
            (json!(null), Appearance::System),
        ] {
            let repaired = Settings::repaired(json!({ "appearance": input.clone() }));
            assert_eq!(repaired.appearance, expected, "{input}");
        }
    }

    #[test]
    fn accent_accepts_its_ten_values_and_falls_back_to_system() {
        for (input, expected) in [
            (json!("system"), Accent::System),
            (json!("orange"), Accent::Orange),
            (json!("gold"), Accent::Gold),
            (json!("red"), Accent::Red),
            (json!("magenta"), Accent::Magenta),
            (json!("purple"), Accent::Purple),
            (json!("indigo"), Accent::Indigo),
            (json!("blue"), Accent::Blue),
            (json!("teal"), Accent::Teal),
            (json!("graphite"), Accent::Graphite),
            (json!("chartreuse"), Accent::System),
            (json!("Teal"), Accent::System),
            (json!(3), Accent::System),
            (json!(null), Accent::System),
        ] {
            let repaired = Settings::repaired(json!({ "accent": input.clone() }));
            assert_eq!(repaired.accent, expected, "{input}");
        }
    }

    #[test]
    fn an_app_setting_key_is_the_name_serde_uses() {
        for setting in [
            AppSetting::DefaultEditor,
            AppSetting::GitClient,
            AppSetting::TerminalApp,
            AppSetting::ConfigEditor,
            AppSetting::DockerStartCommand,
            AppSetting::DockerStopCommand,
        ] {
            assert_eq!(
                serde_json::to_value(setting).unwrap(),
                json!(setting.key()),
                "{setting:?}"
            );
        }
    }

    #[test]
    fn a_bad_field_falls_back_without_touching_the_others() {
        let repaired = Settings::repaired(json!({
            "appearance": "sepia",
            "tags": ["client"],
            "gitClient": "Fork",
            "defaultEditor": 5,
        }));

        assert_eq!(repaired.appearance, Appearance::System);
        assert_eq!(repaired.tags, ["client"]);
        assert_eq!(repaired.git_client, "Fork");
        assert_eq!(repaired.default_editor, "");
    }

    #[test]
    fn tags_are_trimmed_deduped_and_stripped_of_non_strings() {
        let repaired = Settings::repaired(json!({
            "tags": [" client ", "client", "", "   ", 4, null, ["x"], "internal", "internal "],
        }));

        assert_eq!(repaired.tags, ["client", "internal"]);
    }

    #[test]
    fn tags_that_are_not_a_list_become_empty() {
        for raw in [json!("client"), json!({"a": 1}), json!(null)] {
            let repaired = Settings::repaired(json!({ "tags": raw.clone() }));
            assert!(repaired.tags.is_empty(), "{raw}");
        }
    }

    #[test]
    fn shortcuts_keep_null_trim_keys_and_drop_bad_values() {
        let repaired = Settings::repaired(json!({
            "shortcuts": {
                "app:logs": " Cmd+L ",
                "app:data": null,
                "app:quit": "",
                "app:reload": "   ",
                "app:show": 12,
                "app:settings": true,
                " ": "Cmd+X",
                " edit:undo ": "Cmd+Z",
            },
        }));

        assert_eq!(
            repaired.shortcuts,
            BTreeMap::from([
                ("app:logs".to_string(), Some("Cmd+L".to_string())),
                ("app:data".to_string(), None),
                ("edit:undo".to_string(), Some("Cmd+Z".to_string())),
            ])
        );
    }

    #[test]
    fn shortcuts_that_are_not_an_object_become_empty() {
        let repaired = Settings::repaired(json!({ "shortcuts": ["app:logs"] }));
        assert!(repaired.shortcuts.is_empty());
    }

    #[test]
    fn text_settings_are_trimmed_and_may_be_empty() {
        let repaired = Settings::repaired(json!({
            "defaultEditor": "  Zed  ",
            "gitClient": "   ",
            "configEditor": "",
        }));

        assert_eq!(repaired.default_editor, "Zed");
        assert_eq!(repaired.git_client, "");
        assert_eq!(repaired.config_editor, "");
    }

    #[test]
    fn an_empty_or_invalid_terminal_app_falls_back_to_terminal() {
        for raw in [json!(""), json!("   "), json!(null), json!(4)] {
            let repaired = Settings::repaired(json!({ "terminalApp": raw.clone() }));
            assert_eq!(repaired.terminal_app, "Terminal", "{raw}");
        }
    }

    #[test]
    fn unknown_keys_are_dropped_on_the_next_serialization() {
        let repaired = Settings::repaired(json!({ "future": true, "gitClient": "Fork" }));
        assert!(
            serde_json::to_value(&repaired)
                .unwrap()
                .get("future")
                .is_none()
        );
    }

    #[test]
    fn settings_serialize_with_camel_case_fields() {
        assert_eq!(
            serde_json::to_value(Settings::default()).unwrap(),
            json!({
                "appearance": "system",
                "accent": "system",
                "tags": [],
                "shortcuts": {},
                "defaultEditor": "",
                "gitClient": "",
                "terminalApp": "Terminal",
                "configEditor": "",
                "dockerManager": "docker-desktop",
                "dockerStartCommand": "",
                "dockerStopCommand": "",
            })
        );
    }

    #[test]
    fn a_shortcut_without_a_key_serializes_as_null() {
        let settings = Settings::repaired(json!({ "shortcuts": {"app:data": null} }));
        assert_eq!(
            serde_json::to_value(&settings).unwrap()["shortcuts"],
            json!({"app:data": null})
        );
    }
}
