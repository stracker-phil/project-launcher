//! The menu bar's items as the command palette lists them.

use serde::Serialize;
use tauri::{AppHandle, Manager};
use ts_rs::TS;

use super::keys::effective_accelerator;
use super::{PALETTE, menu_items};
use crate::bus::{self, CommandStatus};
use crate::state::AppState;
use crate::store::Shortcuts;

#[derive(Serialize, Clone, Debug, PartialEq, Eq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct MenuCommand {
    /// The menu's title, the palette's section.
    pub menu: String,
    pub label: String,
    pub command: String,
    /// The key equivalent as the menu shows it (`⌃⌘S`).
    pub key: Option<String>,
}

/// Every enabled item of the menus, in menu order, with the owner's keys; Command Palette… itself is left out.
pub fn listed(commands: &dyn CommandStatus, shortcuts: &Shortcuts) -> Vec<MenuCommand> {
    menu_items()
        .filter(|(_, item)| item.command != PALETTE.command && commands.is_enabled(item.command))
        .map(|(menu, item)| MenuCommand {
            menu: menu.to_string(),
            label: item.label.to_string(),
            command: item.command.to_string(),
            key: effective_accelerator(item, shortcuts)
                .as_deref()
                .map(key_label),
        })
        .collect()
}

/// `CmdOrCtrl+Ctrl+S` → `⌃⌘S`: the modifiers in macOS's order, then the key.
fn key_label(accelerator: &str) -> String {
    let (modifiers, key) = accelerator.rsplit_once('+').unwrap_or(("", accelerator));
    let has = |names: &[&str]| {
        modifiers
            .split('+')
            .any(|m| names.iter().any(|n| m.eq_ignore_ascii_case(n)))
    };
    let mut label = String::new();
    for (names, symbol) in [
        (&["Ctrl", "Control"][..], '⌃'),
        (&["Alt", "Option"][..], '⌥'),
        (&["Shift"][..], '⇧'),
        (
            &["CmdOrCtrl", "CommandOrControl", "Cmd", "Command", "Super"][..],
            '⌘',
        ),
    ] {
        if has(names) {
            label.push(symbol);
        }
    }
    label.push_str(&key_symbol(key));
    label
}

fn key_symbol(key: &str) -> String {
    match key {
        "Enter" | "Return" => "↩".into(),
        "Escape" | "Esc" => "⎋".into(),
        "Tab" => "⇥".into(),
        "Backspace" => "⌫".into(),
        "Delete" => "⌦".into(),
        "Space" => "Space".into(),
        "Up" | "ArrowUp" => "↑".into(),
        "Down" | "ArrowDown" => "↓".into(),
        "Left" | "ArrowLeft" => "←".into(),
        "Right" | "ArrowRight" => "→".into(),
        "BracketLeft" => "[".into(),
        "BracketRight" => "]".into(),
        "Comma" => ",".into(),
        "Period" => ".".into(),
        _ => key.to_uppercase(),
    }
}

#[tauri::command]
pub async fn menu_commands(app: AppHandle) -> Vec<MenuCommand> {
    let registry = app.state::<bus::Commands>();
    let shortcuts = app.state::<AppState>().data.settings().shortcuts;
    listed(&registry.available(&app), &shortcuts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bus::CommandBus;

    struct Enabled(&'static [&'static str]);

    impl CommandStatus for Enabled {
        fn knows(&self, id: &str) -> bool {
            self.0.contains(&id)
        }

        fn is_enabled(&self, id: &str) -> bool {
            self.knows(id)
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
        "app:palette",
        "app:reload",
        "app:logs",
        "app:data",
    ];

    fn commands(listed: &[MenuCommand]) -> Vec<&str> {
        listed.iter().map(|item| item.command.as_str()).collect()
    }

    fn find<'a>(listed: &'a [MenuCommand], command: &str) -> &'a MenuCommand {
        listed
            .iter()
            .find(|item| item.command == command)
            .unwrap_or_else(|| panic!("{command} is not listed"))
    }

    #[test]
    fn the_first_item_is_settings_in_the_app_menu_with_its_key() {
        let listed = listed(&Enabled(ALL), &Shortcuts::new());
        assert_eq!(
            listed[0],
            MenuCommand {
                menu: crate::APP_NAME.to_string(),
                label: "Settings…".to_string(),
                command: "app:settings".to_string(),
                key: Some("⌘,".to_string()),
            }
        );
    }

    #[test]
    fn items_follow_menu_order_app_file_edit_view_help() {
        let listed = listed(&Enabled(ALL), &Shortcuts::new());
        let mut menus: Vec<&str> = listed.iter().map(|item| item.menu.as_str()).collect();
        menus.dedup();
        assert_eq!(menus, [crate::APP_NAME, "File", "Edit", "View", "Help"]);
    }

    #[test]
    fn the_edit_menu_lists_undo_and_redo_with_their_keys() {
        let listed = listed(&Enabled(ALL), &Shortcuts::new());
        let edit: Vec<(&str, &str, Option<&str>)> = listed
            .iter()
            .filter(|item| item.menu == "Edit")
            .map(|item| {
                (
                    item.label.as_str(),
                    item.command.as_str(),
                    item.key.as_deref(),
                )
            })
            .collect();
        assert_eq!(
            edit,
            [
                ("Undo", "edit:undo", Some("⌘Z")),
                ("Redo", "edit:redo", Some("⇧⌘Z")),
            ]
        );
    }

    #[test]
    fn every_enabled_item_is_listed_in_menu_order() {
        let listed = listed(&Enabled(ALL), &Shortcuts::new());
        assert_eq!(
            commands(&listed),
            [
                "app:settings",
                "app:quit",
                "window:close",
                "edit:undo",
                "edit:redo",
                "page:view.toggleSidebar",
                "page:view.toggleInspector",
                "app:reload",
                "app:logs",
                "app:data",
            ]
        );
    }

    #[test]
    fn the_command_palette_item_is_never_listed() {
        let listed = listed(&Enabled(ALL), &Shortcuts::new());
        assert!(!commands(&listed).contains(&"app:palette"));
    }

    #[test]
    fn disabled_or_unknown_commands_are_left_out() {
        let listed = listed(
            &Enabled(&["app:settings", "app:logs", "test:unrelated"]),
            &Shortcuts::new(),
        );
        assert_eq!(commands(&listed), ["app:settings", "app:logs"]);
    }

    #[test]
    fn nothing_is_listed_when_no_command_is_enabled() {
        assert!(listed(&Enabled(&[]), &Shortcuts::new()).is_empty());
    }

    #[test]
    fn the_view_menu_lists_the_pane_toggles_with_their_keys() {
        let listed = listed(&Enabled(ALL), &Shortcuts::new());
        let view: Vec<(&str, &str, Option<&str>)> = listed
            .iter()
            .filter(|item| item.menu == "View" && item.command != "app:reload")
            .map(|item| {
                (
                    item.label.as_str(),
                    item.command.as_str(),
                    item.key.as_deref(),
                )
            })
            .collect();
        assert_eq!(
            view,
            [
                ("Toggle Sidebar", "page:view.toggleSidebar", Some("⌃⌘S")),
                ("Toggle Inspector", "page:view.toggleInspector", Some("⌃⌘I")),
            ]
        );
    }

    #[test]
    fn items_without_a_key_equivalent_have_no_key() {
        let listed = listed(&Enabled(ALL), &Shortcuts::new());
        assert_eq!(find(&listed, "app:logs").key, None);
        assert_eq!(find(&listed, "app:data").key, None);
    }

    #[test]
    fn reload_is_listed_from_the_real_bus_in_debug_builds_only() {
        let mut bus = CommandBus::<AppHandle>::default();
        bus::register(&mut bus);

        let listed = listed(&Registered(bus), &Shortcuts::new());

        assert_eq!(
            commands(&listed).contains(&"app:reload"),
            cfg!(debug_assertions)
        );
        assert_eq!(commands(&listed)[0], "app:settings");
    }

    fn overrides(command: &str, key: Option<&str>) -> Shortcuts {
        let mut shortcuts = Shortcuts::new();
        shortcuts.insert(command.to_string(), key.map(str::to_string));
        shortcuts
    }

    #[test]
    fn an_overridden_key_is_shown_as_the_menu_label() {
        for (override_key, expected) in [
            ("Mod+Alt+P", "⌥⌘P"),
            ("cmd+shift+,", "⇧⌘,"),
            ("CmdOrCtrl+Ctrl+Up", "⌃⌘↑"),
            ("Ctrl+Esc", "⌃⎋"),
        ] {
            let listed = listed(&Enabled(ALL), &overrides("app:quit", Some(override_key)));
            assert_eq!(
                find(&listed, "app:quit").key.as_deref(),
                Some(expected),
                "{override_key}"
            );
            assert_eq!(find(&listed, "app:settings").key.as_deref(), Some("⌘,"));
        }
    }

    #[test]
    fn a_null_override_shows_no_key() {
        let listed = listed(&Enabled(ALL), &overrides("app:quit", None));
        assert_eq!(find(&listed, "app:quit").key, None);
        assert_eq!(find(&listed, "edit:undo").key.as_deref(), Some("⌘Z"));
    }

    #[test]
    fn an_override_menus_cannot_take_leaves_the_default_key() {
        let listed = listed(&Enabled(ALL), &overrides("app:quit", Some("Q")));
        assert_eq!(find(&listed, "app:quit").key.as_deref(), Some("⌘Q"));
    }

    #[test]
    fn an_override_gives_a_key_to_an_item_without_one() {
        let listed = listed(&Enabled(ALL), &overrides("app:logs", Some("Mod+Shift+L")));
        assert_eq!(find(&listed, "app:logs").key.as_deref(), Some("⇧⌘L"));
    }

    #[test]
    fn key_labels_put_modifiers_in_macos_order_before_the_key() {
        for (accelerator, expected) in [
            ("CmdOrCtrl+Ctrl+S", "⌃⌘S"),
            ("CmdOrCtrl+Alt+R", "⌥⌘R"),
            ("CmdOrCtrl+,", "⌘,"),
            ("CmdOrCtrl+Q", "⌘Q"),
            ("Shift+Alt+Ctrl+Cmd+X", "⌃⌥⇧⌘X"),
            ("Cmd+Shift+Ctrl+Option+x", "⌃⌥⇧⌘X"),
            ("CommandOrControl+Control+Alt+Shift+k", "⌃⌥⇧⌘K"),
            ("ctrl+shift+a", "⌃⇧A"),
            ("Super+W", "⌘W"),
            ("S", "S"),
        ] {
            assert_eq!(key_label(accelerator), expected, "{accelerator}");
        }
    }

    #[test]
    fn key_labels_use_symbols_for_named_keys() {
        for (accelerator, expected) in [
            ("CmdOrCtrl+Enter", "⌘↩"),
            ("CmdOrCtrl+Return", "⌘↩"),
            ("Escape", "⎋"),
            ("Esc", "⎋"),
            ("CmdOrCtrl+Tab", "⌘⇥"),
            ("CmdOrCtrl+Backspace", "⌘⌫"),
            ("CmdOrCtrl+Delete", "⌘⌦"),
            ("CmdOrCtrl+Space", "⌘Space"),
            ("CmdOrCtrl+Up", "⌘↑"),
            ("CmdOrCtrl+ArrowDown", "⌘↓"),
            ("CmdOrCtrl+Left", "⌘←"),
            ("CmdOrCtrl+ArrowRight", "⌘→"),
            ("CmdOrCtrl+BracketLeft", "⌘["),
            ("CmdOrCtrl+BracketRight", "⌘]"),
            ("CmdOrCtrl+Comma", "⌘,"),
            ("CmdOrCtrl+Period", "⌘."),
        ] {
            assert_eq!(key_label(accelerator), expected, "{accelerator}");
        }
    }

    #[test]
    fn a_menu_command_serializes_with_camel_case_fields() {
        let with_key = MenuCommand {
            menu: "View".to_string(),
            label: "Toggle Sidebar".to_string(),
            command: "page:view.toggleSidebar".to_string(),
            key: Some("⌃⌘S".to_string()),
        };
        let without_key = MenuCommand {
            key: None,
            ..with_key.clone()
        };

        assert_eq!(
            serde_json::to_value(&with_key).unwrap(),
            serde_json::json!({
                "menu": "View",
                "label": "Toggle Sidebar",
                "command": "page:view.toggleSidebar",
                "key": "⌃⌘S",
            })
        );
        assert_eq!(
            serde_json::to_value(&without_key).unwrap()["key"],
            serde_json::Value::Null
        );
    }
}
