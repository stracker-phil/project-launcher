//! The menu items' keys for Settings → Shortcuts, and the check a new key passes before it is stored.

use serde::Serialize;
use tauri::{AppHandle, Manager};
use ts_rs::TS;

use super::keys::{Combo, effective_accelerator, menu_combo};
use super::menu_items;
use crate::bus::{self, CommandStatus};
use crate::error::{AppError, AppResult};
use crate::state::AppState;
use crate::store::Shortcuts;

/// A menu item whose key the owner can change; keys in the page's notation (`Mod+Shift+K`).
#[derive(Serialize, Clone, Debug, PartialEq, Eq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct MenuShortcut {
    pub menu: String,
    pub label: String,
    pub command: String,
    pub default_key: Option<String>,
    /// Effective key, including the owner's override.
    pub key: Option<String>,
}

fn binding(accelerator: Option<&str>) -> Option<String> {
    accelerator
        .and_then(Combo::parse)
        .map(|combo| combo.binding())
}

/// Every item of a known command, enabled or not, in menu order.
pub fn listed(commands: &dyn CommandStatus, shortcuts: &Shortcuts) -> Vec<MenuShortcut> {
    menu_items()
        .filter(|(_, item)| commands.knows(item.command))
        .map(|(menu, item)| MenuShortcut {
            menu: menu.to_string(),
            label: item.label.to_string(),
            command: item.command.to_string(),
            default_key: binding(item.accelerator),
            key: binding(effective_accelerator(item, shortcuts).as_deref()),
        })
        .collect()
}

/// The key to store for a command: a menu item's key must have ⌘ or ⌃ and is stored normalized; any other
/// command's key (a page command's) is kept as the page recorded it.
pub fn checked(command: &str, key: Option<String>) -> AppResult<Option<String>> {
    let Some(key) = key else {
        return Ok(None);
    };
    let command = command.trim();
    if !menu_items().any(|(_, item)| item.command == command) {
        return Ok(Some(key));
    }
    let combo = menu_combo(&key).ok_or_else(|| AppError::InvalidField {
        field: "shortcut".into(),
        message: "A menu shortcut needs ⌘ or ⌃ and a letter, digit or named key".into(),
    })?;
    Ok(Some(combo.binding()))
}

#[tauri::command]
pub fn menu_shortcuts(app: AppHandle) -> Vec<MenuShortcut> {
    let registry = app.state::<bus::Commands>();
    let shortcuts = app.state::<AppState>().data.settings().shortcuts;
    listed(&registry.available(&app), &shortcuts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bus::CommandBus;

    /// Knows the commands but has every one of them disabled.
    struct KnownButDisabled(&'static [&'static str]);

    impl CommandStatus for KnownButDisabled {
        fn knows(&self, id: &str) -> bool {
            self.0.contains(&id)
        }

        fn is_enabled(&self, _id: &str) -> bool {
            false
        }
    }

    struct Registered(CommandBus<AppHandle>);

    impl CommandStatus for Registered {
        fn knows(&self, id: &str) -> bool {
            self.0.knows(id)
        }

        fn is_enabled(&self, id: &str) -> bool {
            self.knows(id)
        }
    }

    const ALL: &[&str] = &[
        "app:settings",
        "app:quit",
        "window:close",
        "edit:undo",
        "edit:redo",
        "page:view.toggleSidebar",
        "page:view.toggleInspector",
        "page:palette.open",
        "app:reload",
        "app:logs",
        "app:data",
    ];

    fn overrides(command: &str, key: Option<&str>) -> Shortcuts {
        let mut shortcuts = Shortcuts::new();
        shortcuts.insert(command.to_string(), key.map(str::to_string));
        shortcuts
    }

    fn find<'a>(listed: &'a [MenuShortcut], command: &str) -> &'a MenuShortcut {
        listed
            .iter()
            .find(|item| item.command == command)
            .unwrap_or_else(|| panic!("{command} is not listed"))
    }

    fn keys(item: &MenuShortcut) -> (Option<&str>, Option<&str>) {
        (item.default_key.as_deref(), item.key.as_deref())
    }

    fn field_of(err: &AppError) -> Option<&str> {
        err.field().map(|f| f.path)
    }

    #[test]
    fn every_known_item_is_listed_in_menu_order_even_when_disabled() {
        let listed = listed(&KnownButDisabled(ALL), &Shortcuts::new());
        let commands: Vec<&str> = listed.iter().map(|item| item.command.as_str()).collect();
        assert_eq!(
            commands,
            [
                "app:settings",
                "app:quit",
                "window:close",
                "edit:undo",
                "edit:redo",
                "page:view.toggleSidebar",
                "page:view.toggleInspector",
                "page:palette.open",
                "app:reload",
                "app:logs",
                "app:data",
            ]
        );
    }

    #[test]
    fn an_item_lists_its_menu_label_and_command() {
        let listed = listed(&KnownButDisabled(ALL), &Shortcuts::new());
        assert_eq!(
            listed[0],
            MenuShortcut {
                menu: crate::APP_NAME.to_string(),
                label: "Settings…".to_string(),
                command: "app:settings".to_string(),
                default_key: Some("Mod+,".to_string()),
                key: Some("Mod+,".to_string()),
            }
        );
        let palette = find(&listed, "page:palette.open");
        assert_eq!(palette.menu, "View");
        assert_eq!(palette.label, "Command Palette…");
    }

    #[test]
    fn default_and_current_keys_are_in_the_pages_notation() {
        let listed = listed(&KnownButDisabled(ALL), &Shortcuts::new());
        for (command, key) in [
            ("app:settings", Some("Mod+,")),
            ("app:quit", Some("Mod+Q")),
            ("edit:undo", Some("Mod+Z")),
            ("edit:redo", Some("Mod+Shift+Z")),
            ("page:view.toggleSidebar", Some("Mod+Ctrl+S")),
            ("page:view.toggleInspector", Some("Mod+Ctrl+I")),
            ("page:palette.open", Some("Mod+K")),
            ("app:reload", Some("Mod+Alt+R")),
            ("app:logs", None),
            ("app:data", None),
        ] {
            assert_eq!(keys(find(&listed, command)), (key, key), "{command}");
        }
    }

    #[test]
    fn items_of_unknown_commands_are_left_out() {
        let listed = listed(
            &KnownButDisabled(&["app:logs", "test:unrelated"]),
            &Shortcuts::new(),
        );
        let commands: Vec<&str> = listed.iter().map(|item| item.command.as_str()).collect();
        assert_eq!(commands, ["app:logs"]);
    }

    #[test]
    fn reload_is_listed_from_the_real_bus_in_debug_builds_only() {
        let mut bus = CommandBus::<AppHandle>::default();
        bus::register(&mut bus);

        let listed = listed(&Registered(bus), &Shortcuts::new());

        let has_reload = listed.iter().any(|item| item.command == "app:reload");
        assert_eq!(has_reload, cfg!(debug_assertions));
        assert_eq!(listed[0].command, "app:settings");
    }

    #[test]
    fn an_override_changes_the_current_key_and_keeps_the_default() {
        let listed = listed(
            &KnownButDisabled(ALL),
            &overrides("page:palette.open", Some("cmd+alt+p")),
        );
        assert_eq!(
            keys(find(&listed, "page:palette.open")),
            (Some("Mod+K"), Some("Mod+Alt+P"))
        );
        assert_eq!(
            keys(find(&listed, "app:quit")),
            (Some("Mod+Q"), Some("Mod+Q"))
        );
    }

    #[test]
    fn a_null_override_leaves_no_current_key_and_keeps_the_default() {
        let listed = listed(&KnownButDisabled(ALL), &overrides("app:quit", None));
        assert_eq!(keys(find(&listed, "app:quit")), (Some("Mod+Q"), None));
    }

    #[test]
    fn an_override_menus_cannot_take_shows_the_default_as_current() {
        let listed = listed(&KnownButDisabled(ALL), &overrides("app:quit", Some("Q")));
        assert_eq!(
            keys(find(&listed, "app:quit")),
            (Some("Mod+Q"), Some("Mod+Q"))
        );
    }

    #[test]
    fn an_override_gives_a_key_to_an_item_without_a_default() {
        let listed = listed(
            &KnownButDisabled(ALL),
            &overrides("app:logs", Some("Ctrl+L")),
        );
        assert_eq!(keys(find(&listed, "app:logs")), (None, Some("Ctrl+L")));
    }

    #[test]
    fn a_menu_key_without_command_or_control_is_refused_as_a_shortcut_field() {
        for key in ["K", "Shift+K", "Alt+K", "Mod+Nope", "Mod+K+L", ""] {
            let err = checked("app:quit", Some(key.to_string())).unwrap_err();
            assert_eq!(field_of(&err), Some("shortcut"), "{key:?}");
        }
    }

    #[test]
    fn a_menu_key_is_stored_normalized_to_the_pages_notation() {
        for (key, expected) in [
            ("cmd+shift+k", "Mod+Shift+K"),
            ("CmdOrCtrl+Shift+K", "Mod+Shift+K"),
            ("Mod+Shift+K", "Mod+Shift+K"),
            ("shift+ctrl+cmd+alt+x", "Mod+Ctrl+Alt+Shift+X"),
            ("Cmd+Comma", "Mod+,"),
            ("Ctrl+Esc", "Ctrl+Escape"),
        ] {
            assert_eq!(
                checked("app:quit", Some(key.to_string())).unwrap(),
                Some(expected.to_string()),
                "{key}"
            );
        }
    }

    #[test]
    fn the_command_is_matched_to_a_menu_item_after_trimming() {
        let err = checked(" app:quit ", Some("K".to_string())).unwrap_err();
        assert_eq!(field_of(&err), Some("shortcut"));
    }

    #[test]
    fn keys_of_page_commands_pass_through_as_recorded() {
        for key in ["K", "cmd+shift+k", "Alt+Down", "Mod+Nope", ""] {
            assert_eq!(
                checked("project.open", Some(key.to_string())).unwrap(),
                Some(key.to_string()),
                "{key:?}"
            );
        }
    }

    #[test]
    fn no_key_passes_through_for_every_command() {
        for command in ["app:quit", "page:palette.open", "project.open"] {
            assert_eq!(checked(command, None).unwrap(), None, "{command}");
        }
    }

    #[test]
    fn a_menu_shortcut_serializes_with_camel_case_fields() {
        let shortcut = MenuShortcut {
            menu: "View".to_string(),
            label: "Toggle Sidebar".to_string(),
            command: "page:view.toggleSidebar".to_string(),
            default_key: Some("Mod+Ctrl+S".to_string()),
            key: None,
        };
        assert_eq!(
            serde_json::to_value(&shortcut).unwrap(),
            serde_json::json!({
                "menu": "View",
                "label": "Toggle Sidebar",
                "command": "page:view.toggleSidebar",
                "defaultKey": "Mod+Ctrl+S",
                "key": null,
            })
        );
    }
}
