//! The menu bar's own items, each with the command it runs, grouped as the menus show them.

use crate::bus::CommandStatus;

/// A menu item that runs a command, with its key equivalent in Tauri's accelerator notation.
#[derive(Debug, PartialEq)]
pub struct Item {
    pub label: &'static str,
    pub command: &'static str,
    pub accelerator: Option<&'static str>,
}

const fn item(label: &'static str, command: &'static str, accelerator: &'static str) -> Item {
    Item {
        label,
        command,
        accelerator: Some(accelerator),
    }
}

const fn plain(label: &'static str, command: &'static str) -> Item {
    Item {
        label,
        command,
        accelerator: None,
    }
}

pub const SETTINGS: Item = item("Settings…", "app:settings", "CmdOrCtrl+,");
pub const QUIT: Item = item("Quit Launcher", "app:quit", "CmdOrCtrl+Q");
pub const CLOSE: Item = item("Close Window", "window:close", "CmdOrCtrl+W");
/// Replace macOS's Undo and Redo; the focused page decides between text and data (`data/edit_menu.rs`).
pub const UNDO_REDO: &[Item] = &[
    item("Undo", "edit:undo", "CmdOrCtrl+Z"),
    item("Redo", "edit:redo", "CmdOrCtrl+Shift+Z"),
];
pub const VIEW_PANES: &[Item] = &[
    item(
        "Toggle Sidebar",
        "page:view.toggleSidebar",
        "CmdOrCtrl+Ctrl+S",
    ),
    item(
        "Toggle Inspector",
        "page:view.toggleInspector",
        "CmdOrCtrl+Ctrl+I",
    ),
];
pub const NEW: &[Item] = &[
    item("New Project…", "page:project.new", "CmdOrCtrl+N"),
    item("New Tag…", "page:tag.new", "CmdOrCtrl+Shift+N"),
];
pub const SEARCH: Item = item("Search Projects", "page:view.search", "CmdOrCtrl+F");
pub const LIBRARY: &[&[Item]] = &[
    &[
        item("All Projects", "page:library.all", "CmdOrCtrl+1"),
        item("Starred", "page:library.starred", "CmdOrCtrl+2"),
    ],
    &[item("Archived", "page:library.archived", "CmdOrCtrl+0")],
];
pub const PROJECT: &[&[Item]] = &[
    &[
        item("Rename Project", "page:project.rename", "CmdOrCtrl+E"),
        plain("Change Icon & Color…", "page:project.icon"),
        plain("Edit Note", "page:project.notes"),
    ],
    &[
        plain("Star Project", "page:project.star"),
        plain("Archive Project", "page:project.archive"),
        item(
            "Lock Project Details",
            "page:project.lock",
            "CmdOrCtrl+Shift+L",
        ),
    ],
    &[
        plain("Open Terminal Here", "page:project.terminal"),
        plain("Reveal in Finder", "page:project.reveal"),
        item("Copy Path", "page:project.copyPath", "CmdOrCtrl+Alt+C"),
        plain("Edit Project File…", "page:project.editFile"),
    ],
    &[plain("Remove from List", "page:project.remove")],
];
pub const PALETTE: Item = item("Command Palette…", "app:palette", "CmdOrCtrl+K");
/// Registered in debug builds only, so release builds leave it out.
pub const RELOAD: Item = item("Reload", "app:reload", "CmdOrCtrl+Alt+R");
pub const HELP: &[Item] = &[
    plain("Open Log Folder", "app:logs"),
    plain("Show Data Folder", "app:data"),
];

/// A menu's title and its own items, in the groups it splits with separators.
pub type MenuDef = (&'static str, &'static [&'static [Item]]);

/// macOS's standard items (About, Services, Hide, …) go around these.
pub const APP_MENU: MenuDef = (crate::APP_NAME, &[&[SETTINGS], &[QUIT]]);
pub const FILE_MENU: MenuDef = ("File", &[NEW, &[CLOSE]]);
/// Undo and Redo above macOS's Cut, Copy, Paste and Select All.
pub const EDIT_MENU: MenuDef = ("Edit", &[UNDO_REDO]);
/// Above macOS's Enter Full Screen.
pub const VIEW_MENU: MenuDef = ("View", &[VIEW_PANES, &[SEARCH, PALETTE, RELOAD]]);
pub const LIBRARY_MENU: MenuDef = ("Library", LIBRARY);
pub const PROJECT_MENU: MenuDef = ("Project", PROJECT);
pub const HELP_MENU: MenuDef = ("Help", &[HELP]);
/// A menu of macOS's standard items only.
pub const WINDOW_TITLE: &str = "Window";
/// Every menu with items of its own, in menu bar order (Window holds only standard items).
pub const MENUS: &[MenuDef] = &[
    APP_MENU,
    FILE_MENU,
    EDIT_MENU,
    VIEW_MENU,
    LIBRARY_MENU,
    PROJECT_MENU,
    HELP_MENU,
];

/// Every item with its menu's title, in menu bar order.
pub fn menu_items() -> impl Iterator<Item = (&'static str, &'static Item)> {
    MENUS.iter().flat_map(|(menu, groups)| {
        groups
            .iter()
            .flat_map(|items| items.iter())
            .map(move |item| (*menu, item))
    })
}

/// The groups as a menu shows them: only items of known commands, and no group left empty.
pub fn shown_groups(
    groups: &'static [&'static [Item]],
    commands: &dyn CommandStatus,
) -> Vec<Vec<&'static Item>> {
    groups
        .iter()
        .map(|items| {
            items
                .iter()
                .filter(|item| commands.knows(item.command))
                .collect::<Vec<_>>()
        })
        .filter(|items| !items.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bus::{self, CommandBus};
    use tauri::AppHandle;

    struct Known(&'static [&'static str]);

    impl CommandStatus for Known {
        fn knows(&self, id: &str) -> bool {
            self.0.contains(&id)
        }

        fn is_enabled(&self, id: &str) -> bool {
            self.knows(id)
        }
    }

    const FIRST: Item = plain("First", "test:first");
    const SECOND: Item = plain("Second", "test:second");
    const THIRD: Item = plain("Third", "test:third");
    static GROUPS: &[&[Item]] = &[&[FIRST, SECOND], &[THIRD]];

    fn labels(groups: &[Vec<&Item>]) -> Vec<Vec<&'static str>> {
        groups
            .iter()
            .map(|group| group.iter().map(|item| item.label).collect())
            .collect()
    }

    fn all_items() -> Vec<&'static Item> {
        MENUS
            .iter()
            .flat_map(|(_, groups)| groups.iter().flat_map(|group| group.iter()))
            .collect()
    }

    #[test]
    fn shown_groups_keep_every_item_when_all_commands_are_known() {
        let known = Known(&["test:first", "test:second", "test:third"]);
        assert_eq!(
            labels(&shown_groups(GROUPS, &known)),
            [vec!["First", "Second"], vec!["Third"]]
        );
    }

    #[test]
    fn shown_groups_leave_out_items_of_unknown_commands() {
        let known = Known(&["test:second", "test:third"]);
        assert_eq!(
            labels(&shown_groups(GROUPS, &known)),
            [vec!["Second"], vec!["Third"]]
        );
    }

    #[test]
    fn shown_groups_drop_a_group_left_without_items() {
        let known = Known(&["test:first"]);
        assert_eq!(labels(&shown_groups(GROUPS, &known)), [vec!["First"]]);
    }

    #[test]
    fn shown_groups_are_empty_when_no_command_is_known() {
        assert!(shown_groups(GROUPS, &Known(&[])).is_empty());
    }

    #[test]
    fn every_menu_item_runs_a_registered_command() {
        let mut bus = CommandBus::<AppHandle>::default();
        bus::register(&mut bus);

        for item in all_items() {
            let expected_missing = item.command == "app:reload" && !cfg!(debug_assertions);
            assert_eq!(
                bus.knows(item.command),
                !expected_missing,
                "{}",
                item.command
            );
        }
    }

    #[test]
    fn every_accelerator_needs_a_modifier() {
        for item in all_items() {
            if let Some(accelerator) = item.accelerator {
                assert!(
                    accelerator.contains("CmdOrCtrl") || accelerator.contains("Ctrl"),
                    "{}: {accelerator}",
                    item.command
                );
            }
        }
    }

    #[test]
    fn page_items_are_only_the_known_window_actions() {
        let mut page_commands: Vec<&str> = all_items()
            .into_iter()
            .map(|item| item.command)
            .filter(|command| command.starts_with("page:"))
            .collect();
        page_commands.sort_unstable();
        assert_eq!(
            page_commands,
            [
                "page:library.all",
                "page:library.archived",
                "page:library.starred",
                "page:project.archive",
                "page:project.copyPath",
                "page:project.editFile",
                "page:project.icon",
                "page:project.lock",
                "page:project.new",
                "page:project.notes",
                "page:project.remove",
                "page:project.rename",
                "page:project.reveal",
                "page:project.star",
                "page:project.terminal",
                "page:tag.new",
                "page:view.search",
                "page:view.toggleInspector",
                "page:view.toggleSidebar",
            ]
        );
    }

    #[test]
    fn page_command_ids_are_unique() {
        let mut seen = std::collections::HashSet::new();
        for item in all_items() {
            if item.command.starts_with("page:") {
                assert!(seen.insert(item.command), "{}", item.command);
            }
        }
    }

    #[test]
    fn no_two_items_share_an_accelerator() {
        let mut seen = std::collections::HashMap::new();
        for item in all_items() {
            if let Some(accelerator) = item.accelerator
                && let Some(other) = seen.insert(accelerator, item.command)
            {
                panic!("{accelerator}: {other} and {}", item.command);
            }
        }
    }

    #[test]
    fn the_menus_follow_the_menu_bar_order() {
        let titles: Vec<&str> = MENUS.iter().map(|(title, _)| *title).collect();
        assert_eq!(
            titles,
            [
                crate::APP_NAME,
                "File",
                "Edit",
                "View",
                "Library",
                "Project",
                "Help"
            ]
        );
    }

    #[test]
    fn the_file_menu_starts_with_new_project_and_new_tag() {
        let found: Vec<_> = menu_items()
            .filter(|(menu, _)| *menu == "File")
            .map(|(_, item)| (item.label, item.command, item.accelerator))
            .collect();
        assert_eq!(
            found,
            [
                ("New Project…", "page:project.new", Some("CmdOrCtrl+N")),
                ("New Tag…", "page:tag.new", Some("CmdOrCtrl+Shift+N")),
                ("Close Window", "window:close", Some("CmdOrCtrl+W")),
            ]
        );
    }
}
